use crate::constants::{
  CLAIM_REWARD_ERROR_REPLY_ID, CONTRACT_NAME, CONTRACT_VERSION, CREATE_REPLY_ID,
};
use crate::error::ContractError;
use crate::state::{CONFIG, VALIDATORS};
use crate::token_factory::{CustomExecuteMsg, DenomUnit, Metadata, TokenExecuteMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
  ensure, Binary, CosmosMsg, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdError, SubMsg,
  Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw_asset::AssetInfoBase;
use cw_utils::parse_instantiate_response_data;
use semver::Version;
use std::collections::HashSet;
use terra_proto_rs::alliance::alliance::{
  MsgClaimDelegationRewards, MsgDelegate, MsgRedelegate, MsgUndelegate,
};
use terra_proto_rs::cosmos::base::v1beta1::Coin;
use terra_proto_rs::traits::Message;
use ve3_shared::adapters::global_config_adapter::ConfigExt;
use ve3_shared::constants::{AT_ASSET_STAKING, AT_DELEGATION_CONTROLLER};
use ve3_shared::contract_connector_alliance::{
  AllianceDelegateMsg, AllianceRedelegateMsg, AllianceUndelegateMsg, CallbackMsg, Config,
  ExecuteMsg, InstantiateMsg, MigrateMsg,
};
use ve3_shared::error::SharedError;
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;
use ve3_shared::extensions::env_ext::EnvExt;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
  let version: Version = CONTRACT_VERSION.parse()?;
  let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

  ensure!(storage_version < version, StdError::generic_err("Invalid contract version"));

  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
  Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> Result<Response<CustomExecuteMsg>, ContractError> {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
  let create_msg = TokenExecuteMsg::CreateDenom {
    subdenom: msg.alliance_token_denom.to_string(),
  };
  let sub_msg = SubMsg::reply_on_success(
    CosmosMsg::Custom(CustomExecuteMsg::Token(create_msg)),
    CREATE_REPLY_ID,
  );

  let config = Config {
    alliance_token_denom: "".to_string(),
    alliance_token_supply: Uint128::zero(),
    reward_denom: msg.reward_denom,
    global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
  };
  CONFIG.save(deps.storage, &config)?;

  VALIDATORS.save(deps.storage, &HashSet::new())?;
  Ok(Response::new().add_attributes(vec![("action", "instantiate")]).add_submessage(sub_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: ExecuteMsg,
) -> Result<Response, ContractError> {
  match msg {
    ExecuteMsg::ClaimRewards {} => claim_rewards(deps, env, info),
    ExecuteMsg::AllianceDelegate(msg) => alliance_delegate(deps, env, info, msg),
    ExecuteMsg::AllianceUndelegate(msg) => alliance_undelegate(deps, env, info, msg),
    ExecuteMsg::AllianceRedelegate(msg) => alliance_redelegate(deps, env, info, msg),
    ExecuteMsg::RemoveValidator {
      validator,
    } => remove_validator(deps, env, info, validator),
    ExecuteMsg::Callback(msg) => callback(deps, env, info, msg),
  }
}

fn callback(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: CallbackMsg,
) -> Result<Response, ContractError> {
  if env.contract.address != info.sender {
    Err(SharedError::UnauthorizedCallbackOnlyCallableByContract {})?
  }

  match msg {
    CallbackMsg::ClaimRewardsCallback {
      asset,
      receiver,
    } => {
      let transfer_msg =
        asset.with_balance_query(&deps.querier, &env.contract.address)?.transfer_msg(receiver)?;

      Ok(
        Response::new()
          .add_attributes(vec![("action", "claim_rewards_callback")])
          .add_message(transfer_msg),
      )
    },
  }
}

fn remove_validator(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  validator: String,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_controller(&deps, &info, &config)?;

  let mut validators = VALIDATORS.load(deps.storage)?;
  validators.remove(&validator);
  VALIDATORS.save(deps.storage, &validators)?;
  Ok(Response::new().add_attributes(vec![("action", "remove_validator")]))
}

fn alliance_delegate(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: AllianceDelegateMsg,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_controller(&deps, &info, &config)?;
  if msg.delegations.is_empty() {
    return Err(ContractError::EmptyDelegation {});
  }
  let mut validators = VALIDATORS.load(deps.storage)?;
  let mut msgs: Vec<CosmosMsg<Empty>> = vec![];
  for delegation in msg.delegations {
    let delegate_msg = MsgDelegate {
      amount: Some(Coin {
        denom: config.alliance_token_denom.clone(),
        amount: delegation.amount.to_string(),
      }),
      delegator_address: env.contract.address.to_string(),
      validator_address: delegation.validator.to_string(),
    };
    msgs.push(CosmosMsg::Stargate {
      type_url: "/alliance.alliance.MsgDelegate".to_string(),
      value: Binary::from(delegate_msg.encode_to_vec()),
    });
    validators.insert(delegation.validator);
  }
  VALIDATORS.save(deps.storage, &validators)?;
  Ok(Response::new().add_attributes(vec![("action", "alliance_delegate")]).add_messages(msgs))
}

fn alliance_undelegate(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: AllianceUndelegateMsg,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_controller(&deps, &info, &config)?;
  if msg.undelegations.is_empty() {
    return Err(ContractError::EmptyDelegation {});
  }
  let mut msgs = vec![];
  for delegation in msg.undelegations {
    let undelegate_msg = MsgUndelegate {
      amount: Some(Coin {
        denom: config.alliance_token_denom.clone(),
        amount: delegation.amount.to_string(),
      }),
      delegator_address: env.contract.address.to_string(),
      validator_address: delegation.validator.to_string(),
    };
    let msg = CosmosMsg::Stargate {
      type_url: "/alliance.alliance.MsgUndelegate".to_string(),
      value: Binary::from(undelegate_msg.encode_to_vec()),
    };
    msgs.push(msg);
  }
  Ok(Response::new().add_attributes(vec![("action", "alliance_undelegate")]).add_messages(msgs))
}

fn alliance_redelegate(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: AllianceRedelegateMsg,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_controller(&deps, &info, &config)?;
  if msg.redelegations.is_empty() {
    return Err(ContractError::EmptyDelegation {});
  }
  let mut msgs = vec![];
  let mut validators = VALIDATORS.load(deps.storage)?;
  for redelegation in msg.redelegations {
    let src_validator = redelegation.src_validator;
    let dst_validator = redelegation.dst_validator;
    let redelegate_msg = MsgRedelegate {
      amount: Some(Coin {
        denom: config.alliance_token_denom.clone(),
        amount: redelegation.amount.to_string(),
      }),
      delegator_address: env.contract.address.to_string(),
      validator_src_address: src_validator.to_string(),
      validator_dst_address: dst_validator.to_string(),
    };
    let msg = CosmosMsg::Stargate {
      type_url: "/alliance.alliance.MsgRedelegate".to_string(),
      value: Binary::from(redelegate_msg.encode_to_vec()),
    };
    msgs.push(msg);
    validators.insert(dst_validator);
  }
  VALIDATORS.save(deps.storage, &validators)?;
  Ok(Response::new().add_attributes(vec![("action", "alliance_redelegate")]).add_messages(msgs))
}

fn claim_rewards(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;

  assert_is_staking(&deps, &info, &config)?;

  let validators = VALIDATORS.load(deps.storage)?;
  let sub_msgs: Vec<SubMsg> = validators
    .iter()
    .map(|v| {
      let msg = MsgClaimDelegationRewards {
        delegator_address: env.contract.address.to_string(),
        validator_address: v.to_string(),
        denom: config.alliance_token_denom.clone(),
      };
      let msg = CosmosMsg::Stargate {
        type_url: "/alliance.alliance.MsgClaimDelegationRewards".to_string(),
        value: Binary::from(msg.encode_to_vec()),
      };
      // Reply on error here is used to ignore errors from claiming rewards with validators that we did not delegate to
      SubMsg::reply_on_error(msg, CLAIM_REWARD_ERROR_REPLY_ID)
    })
    .collect();

  Ok(
    Response::new()
      .add_attributes(vec![("action", "claim_rewards")])
      .add_submessages(sub_msgs)
      .add_message(env.callback_msg(ExecuteMsg::Callback(CallbackMsg::ClaimRewardsCallback {
        asset: AssetInfoBase::Native(config.reward_denom),
        receiver: info.sender,
      }))?),
  )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
  deps: DepsMut,
  env: Env,
  reply: Reply,
) -> Result<Response<CustomExecuteMsg>, ContractError> {
  match reply.id {
    CREATE_REPLY_ID => {
      let response = reply.result.unwrap();
      // It works because the response data is a protobuf encoded string that contains the denom in the first slot (similar to the contract instantiation response)
      let denom = parse_instantiate_response_data(response.data.unwrap().as_slice())
        .map_err(|_| ContractError::Std(StdError::generic_err("parse error".to_string())))?
        .contract_address;
      let total_supply = Uint128::from(1_000_000_000_000_u128);
      let sub_msg_mint =
        SubMsg::new(CosmosMsg::Custom(CustomExecuteMsg::Token(TokenExecuteMsg::MintTokens {
          denom: denom.clone(),
          amount: total_supply,
          mint_to_address: env.contract.address.to_string(),
        })));
      CONFIG.update(deps.storage, |mut config| -> Result<_, ContractError> {
        config.alliance_token_denom = denom.clone();
        config.alliance_token_supply = total_supply;
        Ok(config)
      })?;
      let symbol = "ALLIANCE";

      let sub_msg_metadata =
        SubMsg::new(CosmosMsg::Custom(CustomExecuteMsg::Token(TokenExecuteMsg::SetMetadata {
          denom: denom.clone(),
          metadata: Metadata {
            description: "Staking token for the alliance protocol".to_string(),
            denom_units: vec![DenomUnit {
              denom: denom.clone(),
              exponent: 0,
              aliases: vec![],
            }],
            base: denom.to_string(),
            display: denom.to_string(),
            name: "Alliance Token".to_string(),
            symbol: symbol.to_string(),
          },
        })));
      Ok(
        Response::new()
          .add_attributes(vec![
            ("alliance_token_denom", denom),
            ("alliance_token_total_supply", total_supply.to_string()),
          ])
          .add_submessage(sub_msg_mint)
          .add_submessage(sub_msg_metadata),
      )
    },
    CLAIM_REWARD_ERROR_REPLY_ID => {
      Ok(Response::new().add_attributes(vec![("action", "claim_reward_error")]))
    },
    _ => Err(ContractError::InvalidReplyId(reply.id)),
  }
}

fn assert_is_staking(
  deps: &DepsMut,
  info: &MessageInfo,
  config: &Config,
) -> Result<(), ContractError> {
  config.global_config().assert_has_access(&deps.querier, AT_ASSET_STAKING, &info.sender)?;
  Ok(())
}

// Controller is used to perform administrative operations that deals with delegating the virtual
// tokens to the expected validators
fn assert_controller(
  deps: &DepsMut,
  info: &MessageInfo,
  config: &Config,
) -> Result<(), ContractError> {
  config.global_config().assert_has_access(
    &deps.querier,
    AT_DELEGATION_CONTROLLER,
    &info.sender,
  )?;
  Ok(())
}
