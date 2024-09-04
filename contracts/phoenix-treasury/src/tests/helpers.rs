use crate::contract::{execute, instantiate};
use crate::state::CONFIG;
use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage};
use cosmwasm_std::{DepsMut, Empty, OwnedDeps, Response, StdResult, Uint128};
use ve3_shared::msgs_phoenix_treasury::{
  AllianceDelegateMsg, AllianceDelegation, AllianceRedelegateMsg, AllianceRedelegation,
  AllianceUndelegateMsg, Config, ExecuteMsg, InstantiateMsg,
};

use super::custom_querier::CustomQuerier;

pub const DENOM: &str = "token_factory/token";

pub(super) fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, CustomQuerier, Empty> {
  OwnedDeps {
    storage: MockStorage::default(),
    api: MockApi::default(),
    querier: CustomQuerier::default(),
    custom_query_type: std::marker::PhantomData,
  }
}

#[track_caller]
pub fn setup_contract(deps: DepsMut) -> Response {
  let info = mock_info("admin", &[]);
  let env = mock_env();

  let init_msg = InstantiateMsg {
    alliance_token_denom: "ualliance".to_string(),
    reward_denom: "uluna".to_string(),
    global_config_addr: "global_config".to_string(),
    oracles: vec![],
    vetos: vec![],
    veto_owner: "veto_owner".to_string(),
  };
  instantiate(deps, env, info, init_msg).unwrap()
}

#[track_caller]
pub fn set_alliance_asset(deps: DepsMut) {
  CONFIG
    .update(deps.storage, |c| -> StdResult<_> {
      Ok(Config {
        alliance_token_denom: DENOM.to_string(),
        ..c
      })
    })
    .unwrap();
}

pub fn alliance_delegate(deps: DepsMut, delegations: Vec<(&str, u128)>) -> Response {
  let info = mock_info("controller", &[]);
  let env = mock_env();
  let delegations: Vec<AllianceDelegation> = delegations
    .iter()
    .map(|(addr, amount)| AllianceDelegation {
      validator: addr.to_string(),
      amount: Uint128::new(*amount),
    })
    .collect();
  let msg = ExecuteMsg::AllianceDelegate(AllianceDelegateMsg {
    delegations,
  });
  execute(deps, env, info, msg).unwrap()
}

pub fn alliance_undelegate(deps: DepsMut, delegations: Vec<(&str, u128)>) -> Response {
  let info = mock_info("controller", &[]);
  let env = mock_env();
  let delegations: Vec<AllianceDelegation> = delegations
    .iter()
    .map(|(addr, amount)| AllianceDelegation {
      validator: addr.to_string(),
      amount: Uint128::new(*amount),
    })
    .collect();
  let msg = ExecuteMsg::AllianceUndelegate(AllianceUndelegateMsg {
    undelegations: delegations,
  });
  execute(deps, env, info, msg).unwrap()
}

pub fn alliance_redelegate(deps: DepsMut, redelegations: Vec<(&str, &str, u128)>) -> Response {
  let info = mock_info("controller", &[]);
  let env = mock_env();
  let redelegations: Vec<AllianceRedelegation> = redelegations
    .iter()
    .map(|(src, dst, amount)| AllianceRedelegation {
      src_validator: src.to_string(),
      dst_validator: dst.to_string(),
      amount: Uint128::new(*amount),
    })
    .collect();
  let msg = ExecuteMsg::AllianceRedelegate(AllianceRedelegateMsg {
    redelegations,
  });
  execute(deps, env, info, msg).unwrap()
}
