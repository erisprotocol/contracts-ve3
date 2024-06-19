use crate::contract::execute;
use crate::error::ContractError;
use crate::state::VALIDATORS;
use crate::tests::helpers::{mock_dependencies, set_alliance_asset, setup_contract, DENOM};
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{coin, Addr, Binary, CosmosMsg, Response, SubMsg, Uint128};
use cw_asset::AssetInfo;
use std::collections::HashSet;
use terra_proto_rs::alliance::alliance::MsgClaimDelegationRewards;
use terra_proto_rs::traits::Message;
use ve3_shared::adapters::eris::ErisHub;
use ve3_shared::constants::at_asset_staking;
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;
use ve3_shared::extensions::cosmosmsg_ext::CosmosMsgExt;
use ve3_shared::msgs_connector_alliance::{CallbackMsg, ExecuteMsg};

#[test]
fn test_update_rewards() {
  let mut deps = mock_dependencies();
  deps.querier.set_bank_balances(&[coin(1000000, "uluna")]);
  deps.querier.set_cw20_balance("aluna", MOCK_CONTRACT_ADDR, 100);
  setup_contract(deps.as_mut());
  set_alliance_asset(deps.as_mut());

  VALIDATORS.save(deps.as_mut().storage, &HashSet::from(["validator1".to_string()])).unwrap();

  let res = execute(deps.as_mut(), mock_env(), mock_info("user", &[]), ExecuteMsg::ClaimRewards {})
    .unwrap_err();

  assert_eq!(
    res,
    ContractError::SharedError(ve3_shared::error::SharedError::UnauthorizedMissingRight(
      at_asset_staking("test"),
      "user".to_string()
    ))
  );

  let res =
    execute(deps.as_mut(), mock_env(), mock_info("lp_staking", &[]), ExecuteMsg::ClaimRewards {})
      .unwrap();

  assert_eq!(
    res.messages,
    vec![
      SubMsg::reply_on_error(
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
      ),
      SubMsg::new(
        CallbackMsg::ClaimRewardsCallback {}
          .into_cosmos_msg(&Addr::unchecked("cosmos2contract"))
          .unwrap()
          .to_specific()
          .unwrap()
      ),
      SubMsg::new(
        CallbackMsg::BondRewardsCallback {
          initial: cw_asset::Asset::cw20(Addr::unchecked("aluna"), Uint128::new(100)),
          receiver: Addr::unchecked("lp_staking")
        }
        .into_cosmos_msg(&Addr::unchecked("cosmos2contract"))
        .unwrap()
        .to_specific()
        .unwrap()
      ),
    ]
  );
}

#[test]
fn update_reward_callback() {
  let mut deps = mock_dependencies();
  deps.querier.set_bank_balances(&[coin(2000000, "uluna")]);
  deps.querier.set_cw20_balance("aluna", MOCK_CONTRACT_ADDR, 100);
  setup_contract(deps.as_mut());
  set_alliance_asset(deps.as_mut());

  let res = execute(
    deps.as_mut(),
    mock_env(),
    mock_info("cosmos2contract", &[]),
    ExecuteMsg::Callback(CallbackMsg::ClaimRewardsCallback {}),
  )
  .unwrap();

  assert_eq!(
    res,
    Response::new()
      .add_attributes(vec![
        ("action", "ca/claim_rewards_callback"),
        ("claimed", "native:uluna:2000000")
      ])
      .add_message(
        ErisHub(&Addr::unchecked("hub"))
          .bond_msg(AssetInfo::native("uluna").with_balance_u128(2000000u128), None)
          .unwrap()
      )
  );
}
