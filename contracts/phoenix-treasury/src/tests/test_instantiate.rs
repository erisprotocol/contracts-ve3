use crate::query::query;
use crate::tests::helpers::{mock_dependencies, setup_contract};
use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{from_json, Addr, CosmosMsg, Response, Uint128};
use ve3_shared::helpers::denom::{MsgCreateDenom, MsgMint};
use ve3_shared::msgs_phoenix_treasury::{Config, QueryMsg};

#[test]
fn test_setup_contract() {
  let mut deps = mock_dependencies();
  let res = setup_contract(deps.as_mut());
  let denom = "ualliance";
  let full_denom = format!("factory/{0}/{1}", MOCK_CONTRACT_ADDR, denom);

  let create_msg: CosmosMsg = MsgCreateDenom {
    sender: MOCK_CONTRACT_ADDR.to_string(),
    subdenom: "ualliance".to_string(),
  }
  .into();

  let mint_msg: CosmosMsg = MsgMint {
    sender: MOCK_CONTRACT_ADDR.to_string(),
    amount: Some(ve3_shared::helpers::denom::Coin {
      denom: full_denom.to_string(),
      amount: Uint128::from(1_000_000_000_000_u128).to_string(),
    }),
    mint_to_address: MOCK_CONTRACT_ADDR.to_string(),
  }
  .into();

  assert_eq!(
    res,
    Response::default()
      .add_attributes(vec![
        ("action", "pdt/instantiate"),
        ("alliance_token_denom", &full_denom.to_string()),
        ("alliance_token_total_supply", "1000000000000"),
      ])
      .add_message(create_msg)
      .add_message(mint_msg)
  );

  let query_config = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
  let config: Config = from_json(query_config).unwrap();
  assert_eq!(
    config,
    Config {
      reward_denom: "uluna".to_string(),
      alliance_token_denom: full_denom.to_string(),
      global_config_addr: Addr::unchecked("global_config"),
      vetos: vec![],
    }
  );
}
