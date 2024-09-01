use crate::constants::CLAIM_REWARD_ERROR_REPLY_ID;
use crate::error::ContractError;
use crate::state::{CONFIG, VALIDATORS};
use cosmwasm_std::{Binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, SubMsg};
use terra_proto_rs::alliance::alliance::{
  MsgClaimDelegationRewards, MsgDelegate, MsgRedelegate, MsgUndelegate,
};
use terra_proto_rs::cosmos::base::v1beta1::Coin;
use terra_proto_rs::traits::Message;
use ve3_shared::adapters::global_config_adapter::ConfigExt;
use ve3_shared::constants::AT_DELEGATION_CONTROLLER;
use ve3_shared::msgs_phoenix_treasury::{
  AllianceDelegateMsg, AllianceRedelegateMsg, AllianceUndelegateMsg, Config,
};

pub fn remove_validator(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  validator: String,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_delegation_controller(&deps, &info, &config)?;

  let mut validators = VALIDATORS.load(deps.storage)?;
  validators.remove(&validator);
  VALIDATORS.save(deps.storage, &validators)?;
  Ok(Response::new().add_attributes(vec![("action", "pdt/remove_validator")]))
}

pub fn alliance_delegate(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: AllianceDelegateMsg,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_delegation_controller(&deps, &info, &config)?;
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
  Ok(Response::new().add_attributes(vec![("action", "pdt/alliance_delegate")]).add_messages(msgs))
}

pub fn alliance_undelegate(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: AllianceUndelegateMsg,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_delegation_controller(&deps, &info, &config)?;
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
  Ok(Response::new().add_attributes(vec![("action", "pdt/alliance_undelegate")]).add_messages(msgs))
}

pub fn alliance_redelegate(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: AllianceRedelegateMsg,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_delegation_controller(&deps, &info, &config)?;
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
  Ok(Response::new().add_attributes(vec![("action", "pdt/alliance_redelegate")]).add_messages(msgs))
}

pub fn claim_rewards(
  deps: DepsMut,
  env: Env,
  _info: MessageInfo,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;

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
    Response::new().add_attributes(vec![("action", "pdt/claim_rewards")]).add_submessages(sub_msgs),
  )
}

// Controller is used to perform administrative operations that deals with delegating the virtual
// tokens to the expected validators
fn assert_delegation_controller(
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
