use crate::query::query;
use crate::tests::helpers::setup_contract;
use cosmwasm_std::testing::{mock_dependencies, mock_env};
use cosmwasm_std::{from_json, Addr, Decimal};
use cw_asset::AssetInfoBase;
use ve3_shared::msgs_asset_staking::*;

#[test]
fn test_setup_contract() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());

  // Instantiate is a two steps process that's why
  // alliance_token_denom and alliance_token_supply
  // will be populated on reply.
  let query_config = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
  let config: Config = from_json(query_config).unwrap();
  assert_eq!(
    config,
    Config {
      default_yearly_take_rate: Decimal::percent(10),
      gauge: "stable".to_string(),
      global_config_addr: Addr::unchecked("global_config"),
      reward_info: AssetInfoBase::Native("uluna".to_string()),
    }
  );
}
