use crate::contract::execute;
use crate::error::ContractError;
use crate::state::VALIDATORS;
use crate::tests::helpers::{mock_dependencies, set_alliance_asset, setup_contract, DENOM};
use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
use cosmwasm_std::{coin, Addr, Binary, CosmosMsg, Response, SubMsg};
use cw_asset::AssetInfo;
use std::collections::HashSet;
use terra_proto_rs::alliance::alliance::MsgClaimDelegationRewards;
use terra_proto_rs::traits::Message;
use ve3_shared::constants::at_asset_staking;
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;
use ve3_shared::extensions::cosmosmsg_ext::CosmosMsgExt;
use ve3_shared::msgs_connector_alliance::{CallbackMsg, ExecuteMsg};

#[test]
fn test_update_rewards() {
  let mut deps = mock_dependencies();
  deps.querier.set_bank_balances(&[coin(1000000, "uluna")]);

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
        CallbackMsg::ClaimRewardsCallback {
          asset: cw_asset::AssetInfoBase::Native("uluna".to_string()),
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
  let mut deps = mock_dependencies_with_balance(&[coin(2000000, "uluna")]);
  setup_contract(deps.as_mut());
  set_alliance_asset(deps.as_mut());

  let res = execute(
    deps.as_mut(),
    mock_env(),
    mock_info("cosmos2contract", &[]),
    ExecuteMsg::Callback(CallbackMsg::ClaimRewardsCallback {
      asset: cw_asset::AssetInfoBase::Native("uluna".to_string()),
      receiver: Addr::unchecked("user"),
    }),
  )
  .unwrap();

  assert_eq!(
    res,
    Response::new().add_attributes(vec![("action", "claim_rewards_callback")]).add_message(
      AssetInfo::native("uluna")
        .with_balance_u128(2000000u128)
        .transfer_msg("user".to_string())
        .unwrap()
        .to_specific()
        .unwrap()
    )
  );
}