use crate::contract::execute;
use crate::error::ContractError;
use crate::state::{SHARES, TOTAL};
use crate::tests::helpers::mock_dependencies;
use crate::tests::helpers::{
  query_all_staked_balances, setup_contract, stake, stake_cw20, unstake, unstake_cw20,
  whitelist_assets,
};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::Decimal;
use cosmwasm_std::{coin, to_json_binary, Addr, BankMsg, CosmosMsg, Response, Uint128, WasmMsg};
use cw_asset::{Asset, AssetInfo};
use ve3_shared::msgs_asset_staking::{AssetConfigRuntime, ExecuteMsg, StakedBalanceRes};

#[test]
fn test_stake_cw20() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());
  whitelist_assets(deps.as_mut(), vec![AssetInfo::Cw20(Addr::unchecked("asset1")).into()]);

  let res = stake_cw20(deps.as_mut(), "user1", 100, "asset1");
  assert_eq!(
    res,
    Response::default().add_attributes(vec![
      ("action", "asset/stake"),
      ("user", "user1"),
      ("asset", "cw20:asset1"),
      ("amount", "100"),
      ("share", "100"),
    ])
  );

  let balance = SHARES
    .load(
      deps.as_ref().storage,
      (Addr::unchecked("user1"), &AssetInfo::Cw20(Addr::unchecked("asset1"))),
    )
    .unwrap();
  assert_eq!(balance, Uint128::new(100));

  // Stake more
  let res = stake_cw20(deps.as_mut(), "user1", 100, "asset1");
  assert_eq!(
    res,
    Response::default().add_attributes(vec![
      ("action", "asset/stake"),
      ("user", "user1"),
      ("asset", "cw20:asset1"),
      ("amount", "100"),
      ("share", "100"),
    ])
  );
  let balance = SHARES
    .load(
      deps.as_ref().storage,
      (Addr::unchecked("user1"), &AssetInfo::Cw20(Addr::unchecked("asset1"))),
    )
    .unwrap();
  assert_eq!(balance, Uint128::new(200));

  let total_balance = TOTAL
    .load(deps.as_ref().storage, &AssetInfo::Cw20(Addr::unchecked("asset1")))
    .unwrap();
  assert_eq!(total_balance, (Uint128::new(200), Uint128::new(200)));

  let total_balances_res = query_all_staked_balances(deps.as_ref());
  assert_eq!(
    total_balances_res,
    vec![StakedBalanceRes {
      asset: Asset::cw20(Addr::unchecked("asset1"), Uint128::new(200)),
      shares: Uint128::new(200),
      config: AssetConfigRuntime {
        last_taken_s: 1571797419,
        taken: Uint128::zero(),
        harvested: Uint128::zero(),
        yearly_take_rate: Decimal::percent(10),
        stake_config: ve3_shared::stake_config::StakeConfig::Default
      }
    }]
  );
}

#[test]
fn test_unstake_cw20() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());
  whitelist_assets(deps.as_mut(), vec![AssetInfo::Cw20(Addr::unchecked("asset1")).into()]);

  let res = stake_cw20(deps.as_mut(), "user1", 100, "asset1");
  assert_eq!(
    res,
    Response::default().add_attributes(vec![
      ("action", "asset/stake"),
      ("user", "user1"),
      ("asset", "cw20:asset1"),
      ("amount", "100"),
      ("share", "100"),
    ])
  );

  let res = unstake_cw20(deps.as_mut(), "user1", 50, "asset1");
  assert_eq!(
    res,
    Response::default()
      .add_attributes(vec![
        ("action", "asset/unstake"),
        ("user", "user1"),
        ("asset", "cw20:asset1"),
        ("amount", "50"),
        ("share", "50"),
      ])
      .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "asset1".into(),
        msg: to_json_binary(&cw20::Cw20ExecuteMsg::Transfer {
          recipient: "user1".into(),
          amount: Uint128::new(50),
        })
        .unwrap(),
        funds: vec![],
      }))
  );

  let balance = SHARES
    .load(
      deps.as_ref().storage,
      (Addr::unchecked("user1"), &AssetInfo::Cw20(Addr::unchecked("asset1".to_string()))),
    )
    .unwrap();

  assert_eq!(balance, Uint128::new(50));

  let res = unstake_cw20(deps.as_mut(), "user1", 50, "asset1");
  assert_eq!(
    res,
    Response::default()
      .add_attributes(vec![
        ("action", "asset/unstake"),
        ("user", "user1"),
        ("asset", "cw20:asset1"),
        ("amount", "50"),
        ("share", "50"),
      ])
      .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "asset1".into(),
        msg: to_json_binary(&cw20::Cw20ExecuteMsg::Transfer {
          recipient: "user1".into(),
          amount: Uint128::new(50),
        })
        .unwrap(),
        funds: vec![],
      }))
  );

  let balance = SHARES
    .load(
      deps.as_ref().storage,
      (Addr::unchecked("user1"), &AssetInfo::Cw20(Addr::unchecked("asset1".to_string()))),
    )
    .unwrap();
  assert_eq!(balance, Uint128::new(0));

  let total_balance = TOTAL
    .load(deps.as_ref().storage, &AssetInfo::Cw20(Addr::unchecked("asset1".to_string())))
    .unwrap();
  assert_eq!(total_balance, (Uint128::zero(), Uint128::zero()));
}

#[test]
fn test_stake() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());
  whitelist_assets(deps.as_mut(), vec![AssetInfo::Native("asset1".to_string()).into()]);

  let res = stake(deps.as_mut(), "user1", 100, "asset1");
  assert_eq!(
    res,
    Response::default().add_attributes(vec![
      ("action", "asset/stake"),
      ("user", "user1"),
      ("asset", "native:asset1"),
      ("amount", "100"),
      ("share", "100"),
    ])
  );

  let balance = SHARES
    .load(
      deps.as_ref().storage,
      (Addr::unchecked("user1"), &AssetInfo::Native("asset1".to_string())),
    )
    .unwrap();
  assert_eq!(balance, Uint128::new(100));

  // Stake more
  let res = stake(deps.as_mut(), "user1", 100, "asset1");
  assert_eq!(
    res,
    Response::default().add_attributes(vec![
      ("action", "asset/stake"),
      ("user", "user1"),
      ("asset", "native:asset1"),
      ("amount", "100"),
      ("share", "100"),
    ])
  );
  let balance = SHARES
    .load(
      deps.as_ref().storage,
      (Addr::unchecked("user1"), &AssetInfo::Native("asset1".to_string())),
    )
    .unwrap();
  assert_eq!(balance, Uint128::new(200));

  let total_balance =
    TOTAL.load(deps.as_ref().storage, &AssetInfo::Native("asset1".to_string())).unwrap();
  assert_eq!(total_balance, (Uint128::new(200), Uint128::new(200)));

  let total_balances_res = query_all_staked_balances(deps.as_ref());
  assert_eq!(
    total_balances_res,
    vec![StakedBalanceRes {
      asset: Asset::native("asset1".to_string(), Uint128::new(200)),
      shares: Uint128::new(200),
      config: AssetConfigRuntime {
        last_taken_s: 1571797419,
        taken: Uint128::zero(),
        harvested: Uint128::zero(),
        yearly_take_rate: Decimal::percent(10),
        stake_config: ve3_shared::stake_config::StakeConfig::Default
      }
    }]
  );
}

#[test]
fn test_stake_invalid() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());
  whitelist_assets(deps.as_mut(), vec![AssetInfo::Native("asset1".to_string()).into()]);
  // Stake an unwhitelisted asset
  let msg = ExecuteMsg::Stake {
    recipient: None,
  };
  let info = mock_info("user1", &[coin(100, "asset2")]);
  let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
  assert_eq!(err, ContractError::AssetNotWhitelisted);

  // Stake multiple assets in a single call
  let msg = ExecuteMsg::Stake {
    recipient: None,
  };
  let info = mock_info("user1", &[coin(100, "asset1"), coin(100, "asset2")]);
  let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
  assert_eq!(err, ContractError::OnlySingleAssetAllowed {});

  // Stake nothing in a single call
  let msg = ExecuteMsg::Stake {
    recipient: None,
  };
  let info = mock_info("user1", &[]);
  let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
  assert_eq!(err, ContractError::OnlySingleAssetAllowed {});

  // Stake zero amount
  let msg = ExecuteMsg::Stake {
    recipient: None,
  };
  let info = mock_info("user1", &[coin(0, "asset1")]);
  let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
  assert_eq!(err, ContractError::AmountCannotBeZero {});
}

#[test]
fn test_unstake() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());

  whitelist_assets(deps.as_mut(), vec![AssetInfo::Native("asset1".to_string()).into()]);
  stake(deps.as_mut(), "user1", 100, "asset1");

  let res = unstake(deps.as_mut(), "user1", 50, "asset1");
  assert_eq!(
    res,
    Response::default()
      .add_attributes(vec![
        ("action", "asset/unstake"),
        ("user", "user1"),
        ("asset", "native:asset1"),
        ("amount", "50"),
        ("share", "50"),
      ])
      .add_message(CosmosMsg::Bank(BankMsg::Send {
        to_address: "user1".into(),
        amount: vec![coin(50, "asset1")],
      }))
  );

  let balance = SHARES
    .load(
      deps.as_ref().storage,
      (Addr::unchecked("user1"), &AssetInfo::Native("asset1".to_string())),
    )
    .unwrap();
  assert_eq!(balance, Uint128::new(50));

  let res = unstake(deps.as_mut(), "user1", 50, "asset1");
  assert_eq!(
    res,
    Response::default()
      .add_attributes(vec![
        ("action", "asset/unstake"),
        ("user", "user1"),
        ("asset", "native:asset1"),
        ("amount", "50"),
        ("share", "50"),
      ])
      .add_message(CosmosMsg::Bank(BankMsg::Send {
        to_address: "user1".into(),
        amount: vec![coin(50, "asset1")],
      }))
  );

  let balance = SHARES
    .load(
      deps.as_ref().storage,
      (Addr::unchecked("user1"), &AssetInfo::Native("asset1".to_string())),
    )
    .unwrap();
  assert_eq!(balance, Uint128::new(0));

  let total_balance =
    TOTAL.load(deps.as_ref().storage, &AssetInfo::Native("asset1".to_string())).unwrap();
  assert_eq!(total_balance, (Uint128::new(0), Uint128::new(0)));
}

#[test]
fn test_unstake_invalid() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());

  whitelist_assets(deps.as_mut(), vec![AssetInfo::Native("asset1".to_string()).into()]);
  stake(deps.as_mut(), "user1", 100, "asset1");

  // User does not have any staked asset
  let info = mock_info("user2", &[]);
  let msg = ExecuteMsg::Unstake(Asset::native("asset1", 100u128));
  let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
  assert_eq!(err, ContractError::AmountCannotBeZero {});

  // User unstakes more than they have
  let info = mock_info("user1", &[]);
  let msg = ExecuteMsg::Unstake(Asset::native("asset1", 101u128));
  let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
  assert_eq!(
    res,
    Response::default()
      .add_attributes(vec![
        ("action", "asset/unstake"),
        ("user", "user1"),
        ("asset", "native:asset1"),
        // even though user tries to withdraw 101, he will receive his max (100)
        ("amount", "100"),
        ("share", "100"),
      ])
      .add_message(CosmosMsg::Bank(BankMsg::Send {
        to_address: "user1".into(),
        amount: vec![coin(100, "asset1")],
      }))
  );

  // User unstakes zero amount
  let info = mock_info("user1", &[]);
  let msg = ExecuteMsg::Unstake(Asset::native("asset1", 0u128));
  let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
  assert_eq!(err, ContractError::AmountCannotBeZero {});
}
