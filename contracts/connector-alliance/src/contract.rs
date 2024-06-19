use crate::constants::{CLAIM_REWARD_ERROR_REPLY_ID, CONTRACT_NAME, CONTRACT_VERSION};
use crate::error::ContractError;
use crate::state::{CONFIG, STATE, VALIDATORS};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
  Binary, CosmosMsg, DepsMut, Env, MessageInfo, Reply, Response, SubMsg, Uint128,
};
use cw2::set_contract_version;
use cw_asset::AssetInfo;
use std::collections::HashSet;
use terra_proto_rs::alliance::alliance::{
  MsgClaimDelegationRewards, MsgDelegate, MsgRedelegate, MsgUndelegate,
};
use terra_proto_rs::cosmos::base::v1beta1::Coin;
use terra_proto_rs::traits::Message;
use ve3_shared::adapters::eris::ErisHub;
use ve3_shared::adapters::global_config_adapter::ConfigExt;
use ve3_shared::constants::{at_asset_staking, AT_DELEGATION_CONTROLLER};
use ve3_shared::error::SharedError;
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;
use ve3_shared::extensions::env_ext::EnvExt;
use ve3_shared::helpers::denom::MsgCreateDenom;
use ve3_shared::helpers::take::{compute_balance_amount, compute_share_amount};
use ve3_shared::msgs_connector_alliance::{
  AllianceDelegateMsg, AllianceRedelegateMsg, AllianceUndelegateMsg, CallbackMsg, Config,
  ExecuteMsg, InstantiateMsg, State,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> Result<Response, ContractError> {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  let vt_full_denom = format!("factory/{0}/{1}", env.contract.address, msg.alliance_token_denom);
  let vt_total_supply = Uint128::from(1_000_000_000_000_u128);
  let vt_create_msg: CosmosMsg = MsgCreateDenom {
    sender: env.contract.address.to_string(),
    subdenom: msg.alliance_token_denom.to_string(),
  }
  .into();
  let vt_mint_msg: CosmosMsg = ve3_shared::helpers::denom::MsgMint {
    sender: env.contract.address.to_string(),
    amount: Some(ve3_shared::helpers::denom::Coin {
      denom: vt_full_denom.to_string(),
      amount: vt_total_supply.to_string(),
    }),
    mint_to_address: env.contract.address.to_string(),
  }
  .into();

  let zasset_full_denom = format!("factory/{0}/{1}", env.contract.address, msg.zasset_denom);
  let zasset_create_msg: CosmosMsg = MsgCreateDenom {
    sender: env.contract.address.to_string(),
    subdenom: msg.zasset_denom.to_string(),
  }
  .into();

  let config = Config {
    zasset_denom: zasset_full_denom.clone(),
    alliance_token_denom: vt_full_denom.clone(),
    alliance_token_supply: vt_total_supply,
    reward_denom: msg.reward_denom,
    global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
    gauge: msg.gauge,
    lst_hub_addr: deps.api.addr_validate(&msg.lst_hub_address)?,
    lst_asset_info: msg.lst_asset_info.check(deps.api, None)?,
  };

  let exchange_rate = ErisHub(&config.lst_hub_addr).query_exchange_rate(&deps.querier)?;

  CONFIG.save(deps.storage, &config)?;
  STATE.save(
    deps.storage,
    &State {
      last_exchange_rate: exchange_rate,
      taken: Uint128::zero(),
      harvested: Uint128::zero(),
    },
  )?;

  VALIDATORS.save(deps.storage, &HashSet::new())?;
  Ok(
    Response::new()
      .add_attributes(vec![
        ("action", "ca/instantiate"),
        ("alliance_token_denom", &vt_full_denom),
        ("alliance_token_total_supply", &vt_total_supply.to_string()),
        ("zasset_denom", &zasset_full_denom),
      ])
      .add_message(vt_create_msg)
      .add_message(vt_mint_msg)
      .add_message(zasset_create_msg),
  )
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
    ExecuteMsg::Withdraw {} => withdraw(deps, env, info),
    ExecuteMsg::DistributeRebase {
      update,
    } => distribute_rebase(deps, env, info, update),
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
  mut deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: CallbackMsg,
) -> Result<Response, ContractError> {
  if env.contract.address != info.sender {
    Err(SharedError::UnauthorizedCallbackOnlyCallableByContract {})?
  }

  match msg {
    CallbackMsg::ClaimRewardsCallback {} => {
      let config = CONFIG.load(deps.storage)?;
      let reward_asset = AssetInfo::native(config.reward_denom);
      let received = reward_asset.with_balance_query(&deps.querier, &env.contract.address)?;

      let mut msgs = vec![];
      if !received.amount.is_zero() {
        let bond_msg = ErisHub(&config.lst_hub_addr).bond_msg(received.clone(), None)?;
        msgs.push(bond_msg)
      }

      Ok(
        Response::new()
          .add_attributes(vec![
            ("action", "ca/claim_rewards_callback"),
            ("claimed", &received.to_string()),
          ])
          .add_messages(msgs),
      )
    },
    CallbackMsg::BondRewardsCallback {
      receiver,
      initial,
    } => {
      let config = CONFIG.load(deps.storage)?;

      let zasset = AssetInfo::native(config.zasset_denom.clone());

      let new_amount = initial.info.query_balance(&deps.querier, &env.contract.address)?;
      let added_amount = new_amount.checked_sub(initial.amount)?;
      // let added = initial.info.with_balance(added_amount);

      // println!("bond_rewards {added_amount:?}");
      let (_, stake_available) = _take(&mut deps, &config, initial.amount, true)?;
      let shares = zasset.total_supply(&deps.querier)?;
      let share_amount = compute_share_amount(shares, added_amount, stake_available);

      let mut msgs = vec![];
      if !share_amount.is_zero() {
        let zasset_mint_msg: CosmosMsg = ve3_shared::helpers::denom::MsgMint {
          sender: env.contract.address.to_string(),
          amount: Some(ve3_shared::helpers::denom::Coin {
            denom: config.zasset_denom.to_string(),
            amount: share_amount.to_string(),
          }),
          mint_to_address: receiver.to_string(),
        }
        .into();
        msgs.push(zasset_mint_msg)
      }

      Ok(
        Response::new()
          .add_attributes(vec![
            ("action", "ca/bond_rewards_callback"),
            ("amount", &added_amount.to_string()),
            ("share", &share_amount.to_string()),
          ])
          .add_messages(msgs),
      )
    },
  }
}

fn _take(
  deps: &mut DepsMut,
  config: &Config,
  stake_in_contract: Uint128,
  save: bool,
) -> Result<(State, Uint128), ContractError> {
  let mut state = STATE.load(deps.storage)?;

  let last_exchange_rate = state.last_exchange_rate;
  let current_exchange_rate = ErisHub(&config.lst_hub_addr).query_exchange_rate(&deps.querier)?;

  let stake_available = stake_in_contract.checked_add(state.harvested)?.checked_sub(state.taken)?;

  if current_exchange_rate.le(&last_exchange_rate) {
    return Ok((state, stake_available));
  }

  // no check needed, as we checked for "le" already. current_exchange_rate is also not zero
  let exchange_rate_diff = (current_exchange_rate - last_exchange_rate) / current_exchange_rate;

  let stake_to_extract = exchange_rate_diff * stake_available;
  state.taken = state.taken.checked_add(stake_to_extract)?;
  state.last_exchange_rate = current_exchange_rate;

  // println!("new_state {state:?}");

  if save {
    STATE.save(deps.storage, &state)?;
  }

  let stake_available = stake_in_contract.checked_add(state.harvested)?.checked_sub(state.taken)?;

  Ok((state, stake_available))
}

fn distribute_rebase(
  mut deps: DepsMut,
  env: Env,
  _info: MessageInfo,
  update: Option<bool>,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;

  let mut state = if update == Some(true) {
    let stake_balance =
      config.lst_asset_info.with_balance_query(&deps.querier, &env.contract.address)?;
    let (state, _) = _take(&mut deps, &config, stake_balance.amount, false)?;
    state
  } else {
    STATE.load(deps.storage)?
  };

  let take_amount = state.taken.checked_sub(state.harvested)?;
  if take_amount.is_zero() {
    return Err(ContractError::NothingToTake);
  }

  state.harvested = state.taken;
  STATE.save(deps.storage, &state)?;

  let take_asset = config.lst_asset_info.with_balance(take_amount);

  let asset_gauge = config.asset_gauge(&deps.querier)?;
  let rebase_msg = asset_gauge.add_rebase_msg(take_asset.clone())?;

  Ok(
    Response::new()
      .add_attributes(vec![("action", "ca/distribute_rebase"), ("taken", &take_asset.to_string())])
      .add_message(rebase_msg),
  )
}

fn withdraw(mut deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
  let contract_addr = env.contract.address.clone();
  let config = CONFIG.load(deps.storage)?;
  let zasset = AssetInfo::native(config.zasset_denom.clone());
  let received = zasset.assert_received(&info)?;
  let available = config.lst_asset_info.query_balance(&deps.querier, contract_addr.clone())?;

  let (_, asset_available) = _take(&mut deps, &config, available, true)?;

  let share_amount = received.amount;
  let shares = zasset.total_supply(&deps.querier)?;
  let withdraw_amount = compute_balance_amount(shares, share_amount, asset_available);

  let transfer_msg =
    config.lst_asset_info.with_balance(withdraw_amount).transfer_msg(&info.sender)?;

  let burn_msg: CosmosMsg = ve3_shared::helpers::denom::MsgBurn {
    sender: contract_addr.to_string(),
    amount: Some(ve3_shared::helpers::denom::Coin {
      denom: config.zasset_denom.to_string(),
      amount: share_amount.to_string(),
    }),
    burn_from_address: contract_addr.to_string(),
  }
  .into();

  Ok(
    Response::new()
      .add_attributes(vec![
        ("action", "ca/withdraw"),
        ("amount", &withdraw_amount.to_string()),
        ("share", &share_amount.to_string()),
      ])
      .add_message(burn_msg)
      .add_message(transfer_msg),
  )
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
  Ok(Response::new().add_attributes(vec![("action", "ca/remove_validator")]))
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
  let mut msgs: Vec<CosmosMsg> = vec![];
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
  Ok(Response::new().add_attributes(vec![("action", "ca/alliance_delegate")]).add_messages(msgs))
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
  Ok(Response::new().add_attributes(vec![("action", "ca/alliance_undelegate")]).add_messages(msgs))
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
  Ok(Response::new().add_attributes(vec![("action", "ca/alliance_redelegate")]).add_messages(msgs))
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
      .add_attributes(vec![("action", "ca/claim_rewards")])
      .add_submessages(sub_msgs)
      .add_message(env.callback_msg(ExecuteMsg::Callback(CallbackMsg::ClaimRewardsCallback {}))?)
      .add_message(env.callback_msg(ExecuteMsg::Callback(CallbackMsg::BondRewardsCallback {
        initial: config.lst_asset_info.with_balance_query(&deps.querier, &env.contract.address)?,
        receiver: info.sender,
      }))?),
  )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
  match reply.id {
    CLAIM_REWARD_ERROR_REPLY_ID => {
      Ok(Response::new().add_attributes(vec![("action", "ca/claim_reward_error")]))
    },
    _ => Err(ContractError::InvalidReplyId(reply.id)),
  }
}

fn assert_is_staking(
  deps: &DepsMut,
  info: &MessageInfo,
  config: &Config,
) -> Result<(), ContractError> {
  config.global_config().assert_has_access(
    &deps.querier,
    &at_asset_staking(&config.gauge),
    &info.sender,
  )?;
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
