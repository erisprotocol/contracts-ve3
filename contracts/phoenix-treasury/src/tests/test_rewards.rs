use crate::contract::execute;
use crate::state::VALIDATORS;
use crate::tests::helpers::{mock_dependencies, set_alliance_asset, setup_contract, DENOM};
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{coin, Binary, CosmosMsg, SubMsg};
use std::collections::HashSet;
use terra_proto_rs::alliance::alliance::MsgClaimDelegationRewards;
use terra_proto_rs::traits::Message;
use ve3_shared::extensions::cosmosmsg_ext::CosmosMsgExt;
use ve3_shared::msgs_phoenix_treasury::ExecuteMsg;

#[test]
fn test_update_rewards() {
  let mut deps = mock_dependencies();
  deps.querier.set_bank_balances(&[coin(1000000, "uluna")]);
  deps.querier.set_cw20_balance("aluna", MOCK_CONTRACT_ADDR, 100);
  setup_contract(deps.as_mut());
  set_alliance_asset(deps.as_mut());

  VALIDATORS.save(deps.as_mut().storage, &HashSet::from(["validator1".to_string()])).unwrap();

  let _ = execute(deps.as_mut(), mock_env(), mock_info("user", &[]), ExecuteMsg::ClaimRewards {})
    .unwrap();

  let res =
    execute(deps.as_mut(), mock_env(), mock_info("lp_staking", &[]), ExecuteMsg::ClaimRewards {})
      .unwrap();

  assert_eq!(
    res.messages,
    vec![SubMsg::reply_on_error(
      CosmosMsg::Stargate {
        type_url: "/alliance.alliance.MsgClaimDelegationRewards".to_string(),
        value: Binary::from(
          MsgClaimDelegationRewards {
            delegator_address: "cosmos2contract".to_string(),
            validator_address: "validator1".to_string(),
            denom: DENOM.to_string(),
          }
          .encode_to_vec()
        ),
      }
      .to_specific()
      .unwrap(),
      2,
    )]
  );
}
