use crate::contract::execute;
use crate::error::ContractError;
use crate::state::{CONFIG, VALIDATORS};
use crate::tests::helpers::{
  alliance_delegate, alliance_redelegate, alliance_undelegate, mock_dependencies, setup_contract,
};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{Binary, CosmosMsg, StdResult, SubMsg, Uint128};
use std::collections::HashSet;
use terra_proto_rs::alliance::alliance::{MsgDelegate, MsgRedelegate};
use terra_proto_rs::cosmos::base::v1beta1::Coin;
use terra_proto_rs::traits::Message;
use ve3_shared::constants::AT_DELEGATION_CONTROLLER;
use ve3_shared::error::SharedError;
use ve3_shared::msgs_phoenix_treasury::{
  AllianceDelegateMsg, AllianceDelegation, AllianceUndelegateMsg, Config, ExecuteMsg,
};

#[test]
fn test_alliance_delegate() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());

  let denom = "token_factory/token";
  // set alliance token denom
  CONFIG
    .update(deps.as_mut().storage, |c| -> StdResult<_> {
      Ok(Config {
        alliance_token_denom: denom.to_string(),
        ..c
      })
    })
    .unwrap();
  let res = alliance_delegate(deps.as_mut(), vec![("validator1", 100), ("validator2", 400)]);

  let delegate_msg_1 = MsgDelegate {
    amount: Some(Coin {
      denom: denom.to_string(),
      amount: "100".to_string(),
    }),
    delegator_address: "cosmos2contract".to_string(),
    validator_address: "validator1".to_string(),
  };
  let delegate_msg_2 = MsgDelegate {
    amount: Some(Coin {
      denom: denom.to_string(),
      amount: "400".to_string(),
    }),
    delegator_address: "cosmos2contract".to_string(),
    validator_address: "validator2".to_string(),
  };

  assert_eq!(res.messages.len(), 2);
  assert_eq!(
    res.messages,
    vec![
      SubMsg::new(CosmosMsg::Stargate {
        type_url: "/alliance.alliance.MsgDelegate".to_string(),
        value: Binary::from(delegate_msg_1.encode_to_vec()),
      }),
      SubMsg::new(CosmosMsg::Stargate {
        type_url: "/alliance.alliance.MsgDelegate".to_string(),
        value: Binary::from(delegate_msg_2.encode_to_vec()),
      }),
    ]
  );

  let validators = VALIDATORS.load(deps.as_ref().storage).unwrap();
  assert_eq!(validators, HashSet::from(["validator1".to_string(), "validator2".to_string()]));
}

#[test]
fn test_alliance_delegation_invalid() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());
  let denom = "token_factory/token";
  // set alliance token denom
  CONFIG
    .update(deps.as_mut().storage, |c| -> StdResult<_> {
      Ok(Config {
        alliance_token_denom: denom.to_string(),
        ..c
      })
    })
    .unwrap();
  let info = mock_info("user", &[]);
  let msg = AllianceDelegateMsg {
    delegations: vec![AllianceDelegation {
      validator: "validator1".to_string(),
      amount: Uint128::new(100),
    }],
  };
  let err =
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::AllianceDelegate(msg)).unwrap_err();
  assert_eq!(
    err,
    ContractError::SharedError(SharedError::UnauthorizedMissingRight(
      AT_DELEGATION_CONTROLLER.to_string(),
      "user".to_string()
    ))
  );

  let info = mock_info("controller", &[]);
  let msg = AllianceDelegateMsg {
    delegations: vec![],
  };
  let err =
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::AllianceDelegate(msg)).unwrap_err();
  assert_eq!(err, ContractError::EmptyDelegation {});
}

#[test]
fn test_alliance_undelegate() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());
  let denom = "token_factory/token";
  // set alliance token denom
  CONFIG
    .update(deps.as_mut().storage, |c| -> StdResult<_> {
      Ok(Config {
        alliance_token_denom: denom.to_string(),
        ..c
      })
    })
    .unwrap();

  let res = alliance_undelegate(deps.as_mut(), vec![("validator1", 100), ("validator2", 400)]);

  assert_eq!(res.messages.len(), 2);
  assert_eq!(
    res.messages,
    vec![
      SubMsg::new(CosmosMsg::Stargate {
        type_url: "/alliance.alliance.MsgUndelegate".to_string(),
        value: Binary::from(
          MsgDelegate {
            amount: Some(Coin {
              denom: denom.to_string(),
              amount: "100".to_string(),
            }),
            delegator_address: "cosmos2contract".to_string(),
            validator_address: "validator1".to_string(),
          }
          .encode_to_vec()
        ),
      }),
      SubMsg::new(CosmosMsg::Stargate {
        type_url: "/alliance.alliance.MsgUndelegate".to_string(),
        value: Binary::from(
          MsgDelegate {
            amount: Some(Coin {
              denom: denom.to_string(),
              amount: "400".to_string(),
            }),
            delegator_address: "cosmos2contract".to_string(),
            validator_address: "validator2".to_string(),
          }
          .encode_to_vec()
        ),
      }),
    ]
  );
}

#[test]
fn test_alliance_undelegation_invalid() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());
  let denom = "token_factory/token";
  // set alliance token denom
  CONFIG
    .update(deps.as_mut().storage, |c| -> StdResult<_> {
      Ok(Config {
        alliance_token_denom: denom.to_string(),
        ..c
      })
    })
    .unwrap();
  let info = mock_info("user", &[]);
  let msg = AllianceUndelegateMsg {
    undelegations: vec![AllianceDelegation {
      validator: "validator1".to_string(),
      amount: Uint128::new(100),
    }],
  };
  let err =
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::AllianceUndelegate(msg)).unwrap_err();
  assert_eq!(
    err,
    ContractError::SharedError(SharedError::UnauthorizedMissingRight(
      AT_DELEGATION_CONTROLLER.to_string(),
      "user".to_string()
    ))
  );

  let info = mock_info("controller", &[]);
  let msg = AllianceUndelegateMsg {
    undelegations: vec![],
  };
  let err =
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::AllianceUndelegate(msg)).unwrap_err();
  assert_eq!(err, ContractError::EmptyDelegation {});
}

#[test]
fn test_alliance_redelegate() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());
  let denom = "token_factory/token";
  // set alliance token denom
  CONFIG
    .update(deps.as_mut().storage, |c| -> StdResult<_> {
      Ok(Config {
        alliance_token_denom: denom.to_string(),
        ..c
      })
    })
    .unwrap();

  let res = alliance_redelegate(
    deps.as_mut(),
    vec![("validator1", "validator2", 100), ("validator2", "validator3", 400)],
  );

  assert_eq!(res.messages.len(), 2);
  assert_eq!(
    res.messages,
    vec![
      SubMsg::new(CosmosMsg::Stargate {
        type_url: "/alliance.alliance.MsgRedelegate".to_string(),
        value: Binary::from(
          MsgRedelegate {
            amount: Some(Coin {
              denom: denom.to_string(),
              amount: "100".to_string(),
            }),
            delegator_address: "cosmos2contract".to_string(),
            validator_src_address: "validator1".to_string(),
            validator_dst_address: "validator2".to_string(),
          }
          .encode_to_vec()
        ),
      }),
      SubMsg::new(CosmosMsg::Stargate {
        type_url: "/alliance.alliance.MsgRedelegate".to_string(),
        value: Binary::from(
          MsgRedelegate {
            amount: Some(Coin {
              denom: denom.to_string(),
              amount: "400".to_string(),
            }),
            delegator_address: "cosmos2contract".to_string(),
            validator_src_address: "validator2".to_string(),
            validator_dst_address: "validator3".to_string(),
          }
          .encode_to_vec()
        ),
      }),
    ]
  );
  let validators = VALIDATORS.load(deps.as_ref().storage).unwrap();
  assert_eq!(validators, HashSet::from(["validator2".to_string(), "validator3".to_string()]));
}
