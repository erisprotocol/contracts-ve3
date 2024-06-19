use crate::contract::execute;
use crate::error::ContractError;
use crate::state::{ASSET_REWARD_DISTRIBUTION, ASSET_REWARD_RATE, TOTAL, USER_ASSET_REWARD_RATE};
use crate::tests::helpers::{
  asset_distribution_1, asset_distribution_2, asset_distribution_broken_1,
  asset_distribution_broken_2, claim_rewards, mock_dependencies, query_all_rewards,
  query_asset_reward_distribution, query_rewards, setup_contract, stake, unstake, whitelist_assets,
};
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
  coin, coins, to_json_binary, Addr, BankMsg, CosmosMsg, Decimal, Response, Uint128, WasmMsg,
};
use cw_asset::{Asset, AssetInfo};
use ve3_shared::error::SharedError;
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;
use ve3_shared::{msgs_asset_staking::*, msgs_connector_alliance};

#[test]
fn test_update_rewards() {
  let mut deps = mock_dependencies();
  deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(1000000, "uluna")]);
  setup_contract(deps.as_mut());

  let res =
    execute(deps.as_mut(), mock_env(), mock_info("user", &[]), ExecuteMsg::UpdateRewards {})
      .unwrap();

  assert_eq!(
    res.messages[0].msg,
    CosmosMsg::Wasm(WasmMsg::Execute {
      funds: vec![],
      contract_addr: "connector".to_string(),
      msg: to_json_binary(&msgs_connector_alliance::ExecuteMsg::ClaimRewards {}).unwrap(),
    })
  );

  assert_eq!(
    res.messages[1].msg,
    CosmosMsg::Wasm(WasmMsg::Execute {
      funds: vec![],
      contract_addr: MOCK_CONTRACT_ADDR.to_string(),
      msg: to_json_binary(&ExecuteMsg::Callback(CallbackMsg::UpdateRewards {
        initial_balance: Asset::native("uluna", 1000000u128)
      }))
      .unwrap(),
    })
  );
}

#[test]
fn test_update_rewards_with_funds_sent() {
  let mut deps = mock_dependencies();
  deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(1000000, "uluna")]);

  setup_contract(deps.as_mut());

  let res = execute(
    deps.as_mut(),
    mock_env(),
    mock_info("user", &[coin(1000000, "uluna")]),
    ExecuteMsg::UpdateRewards {},
  )
  .unwrap_err();

  assert_eq!(res, ContractError::SharedError(ve3_shared::error::SharedError::NoFundsAllowed {}));
}

#[test]
fn update_reward_callback() {
  let mut deps = mock_dependencies();
  deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(2000000, "uluna")]);
  setup_contract(deps.as_mut());

  TOTAL
    .save(
      deps.as_mut().storage,
      &AssetInfo::Native("aWHALE".to_string()),
      &(Uint128::new(1000000), Uint128::new(1000000)),
    )
    .unwrap();

  TOTAL
    .save(
      deps.as_mut().storage,
      &AssetInfo::Native("bWHALE".to_string()),
      &(Uint128::new(100000), Uint128::new(100000)),
    )
    .unwrap();

  ASSET_REWARD_DISTRIBUTION
    .save(
      deps.as_mut().storage,
      &vec![
        AssetDistribution {
          asset: AssetInfo::Native("aWHALE".to_string()),
          distribution: Decimal::percent(10),
          total_vp: Uint128::zero(),
        },
        AssetDistribution {
          asset: AssetInfo::Native("bWHALE".to_string()),
          distribution: Decimal::percent(60),
          total_vp: Uint128::zero(),
        },
        AssetDistribution {
          asset: AssetInfo::Native("aMONKEY".to_string()),
          distribution: Decimal::percent(30),
          total_vp: Uint128::zero(),
        },
      ],
    )
    .unwrap();

  let res = execute(
    deps.as_mut(),
    mock_env(),
    mock_info("any", &[]),
    ExecuteMsg::Callback(CallbackMsg::UpdateRewards {
      initial_balance: AssetInfo::native("uluna").with_balance(Uint128::new(1000000u128)),
    }),
  )
  .unwrap_err();

  assert_eq!(res, SharedError::UnauthorizedCallbackOnlyCallableByContract {}.into());

  let res = execute(
    deps.as_mut(),
    mock_env(),
    mock_info(MOCK_CONTRACT_ADDR, &[]),
    ExecuteMsg::Callback(CallbackMsg::UpdateRewards {
      initial_balance: AssetInfo::native("uluna").with_balance(Uint128::new(1000000u128)),
    }),
  )
  .unwrap();

  let a_whale_rate = ASSET_REWARD_RATE
    .load(deps.as_ref().storage, &AssetInfo::Native("aWHALE".to_string()))
    .unwrap();

  assert_eq!(a_whale_rate, Decimal::from_atomics(Uint128::one(), 1).unwrap());
  let b_whale_rate = ASSET_REWARD_RATE
    .load(deps.as_ref().storage, &AssetInfo::Native("bWHALE".to_string()))
    .unwrap();
  assert_eq!(b_whale_rate, Decimal::from_atomics(Uint128::new(6), 0).unwrap());
  ASSET_REWARD_RATE
    .load(deps.as_ref().storage, &AssetInfo::Native("cMONKEY".to_string()))
    .unwrap_err();

  assert_eq!(
    res,
    Response::new().add_attributes(vec![
      ("action", "asset/update_rewards_callback"),
      ("rewards", "native:uluna:1000000")
    ])
  );
}

#[test]
fn claim_user_rewards() {
  let mut deps = mock_dependencies();
  deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(2000000, "uluna")]);
  setup_contract(deps.as_mut());
  whitelist_assets(deps.as_mut(), vec![AssetInfo::Native("aWHALE".to_string()).into()]);
  stake(deps.as_mut(), "user1", 1000000, "aWHALE");
  stake(deps.as_mut(), "user2", 4000000, "aWHALE");

  ASSET_REWARD_DISTRIBUTION
    .save(
      deps.as_mut().storage,
      &vec![
        AssetDistribution {
          asset: AssetInfo::Native("aWHALE".to_string()),
          distribution: Decimal::percent(50),
          total_vp: Uint128::zero(),
        },
        AssetDistribution {
          asset: AssetInfo::Native("bWHALE".to_string()),
          distribution: Decimal::percent(50),
          total_vp: Uint128::zero(),
        },
      ],
    )
    .unwrap();

  execute(
    deps.as_mut(),
    mock_env(),
    mock_info(MOCK_CONTRACT_ADDR, &[]),
    ExecuteMsg::Callback(CallbackMsg::UpdateRewards {
      initial_balance: AssetInfo::native("uluna").with_balance(Uint128::new(1000000u128)),
    }),
  )
  .unwrap();

  let rewards = query_rewards(deps.as_ref(), "user1", "aWHALE");
  assert_eq!(
    rewards,
    PendingRewardsRes {
      reward_asset: Asset::native("uluna".to_string(), 100000u128),
      staked_asset_share: Asset::native("aWHALE".to_string(), 1000000u128),
    }
  );

  let all_rewards = query_all_rewards(deps.as_ref(), "user1");
  assert_eq!(
    all_rewards,
    vec![PendingRewardsRes {
      reward_asset: Asset::native("uluna".to_string(), 100000u128),
      staked_asset_share: Asset::native("aWHALE".to_string(), 1000000u128),
    }]
  );

  let res = claim_rewards(deps.as_mut(), "user1", "aWHALE");
  assert_eq!(
    res,
    Response::new()
      .add_attributes(vec![
        ("action", "asset/claim_rewards"),
        ("user", "user1"),
        ("assets", "native:aWHALE"),
        ("reward_amount", "100000"),
      ])
      .add_message(CosmosMsg::Bank(BankMsg::Send {
        to_address: "user1".to_string(),
        amount: coins(100000, "uluna"),
      }))
  );

  let user_reward_rate = USER_ASSET_REWARD_RATE
    .load(
      deps.as_ref().storage,
      (Addr::unchecked("user1"), &AssetInfo::Native("aWHALE".to_string())),
    )
    .unwrap();
  let asset_reward_rate = ASSET_REWARD_RATE
    .load(deps.as_ref().storage, &AssetInfo::Native("aWHALE".to_string()))
    .unwrap();
  assert_eq!(user_reward_rate, asset_reward_rate);

  let rewards = query_rewards(deps.as_ref(), "user1", "aWHALE");
  assert_eq!(
    rewards,
    PendingRewardsRes {
      reward_asset: Asset::native("uluna".to_string(), 0u128),
      staked_asset_share: Asset::native("aWHALE".to_string(), 1000000u128),
    }
  );

  let all_rewards = query_all_rewards(deps.as_ref(), "user1");
  assert_eq!(all_rewards, vec![]);

  let res = claim_rewards(deps.as_mut(), "user1", "aWHALE");
  assert_eq!(
    res,
    Response::new().add_attributes(vec![
      ("action", "asset/claim_rewards"),
      ("user", "user1"),
      ("assets", "native:aWHALE"),
      ("reward_amount", "0"),
    ])
  );

  // Update more rewards
  deps
    .querier
    .bank_querier
    .update_balance(MOCK_CONTRACT_ADDR, vec![coin(1900000 + 100000, "uluna")]);

  execute(
    deps.as_mut(),
    mock_env(),
    mock_info(MOCK_CONTRACT_ADDR, &[]),
    ExecuteMsg::Callback(CallbackMsg::UpdateRewards {
      initial_balance: AssetInfo::native("uluna").with_balance(Uint128::new(1900000u128)),
    }),
  )
  .unwrap();
  let res = claim_rewards(deps.as_mut(), "user1", "aWHALE");
  assert_eq!(
    res,
    Response::new()
      .add_attributes(vec![
        ("action", "asset/claim_rewards"),
        ("user", "user1"),
        ("assets", "native:aWHALE"),
        ("reward_amount", "10000"),
      ])
      .add_message(CosmosMsg::Bank(BankMsg::Send {
        to_address: "user1".to_string(),
        amount: coins(10000, "uluna"),
      }))
  );
}

#[test]
fn claim_user_rewards_after_staking() {
  let mut deps = mock_dependencies();
  deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(2000000, "uluna")]);
  setup_contract(deps.as_mut());

  whitelist_assets(deps.as_mut(), vec![AssetInfo::Native("aWHALE".to_string()).into()]);
  stake(deps.as_mut(), "user1", 1000000, "aWHALE");
  stake(deps.as_mut(), "user2", 4000000, "aWHALE");

  ASSET_REWARD_DISTRIBUTION
    .save(
      deps.as_mut().storage,
      &vec![
        AssetDistribution {
          asset: AssetInfo::Native("aWHALE".to_string()),
          distribution: Decimal::percent(50),
          total_vp: Uint128::zero(),
        },
        AssetDistribution {
          asset: AssetInfo::Native("bWHALE".to_string()),
          distribution: Decimal::percent(50),
          total_vp: Uint128::zero(),
        },
      ],
    )
    .unwrap();

  execute(
    deps.as_mut(),
    mock_env(),
    mock_info(MOCK_CONTRACT_ADDR, &[]),
    ExecuteMsg::Callback(CallbackMsg::UpdateRewards {
      initial_balance: AssetInfo::native("uluna").with_balance(Uint128::new(1000000)),
    }),
  )
  .unwrap();

  stake(deps.as_mut(), "user1", 1000000, "aWHALE");

  let res = claim_rewards(deps.as_mut(), "user1", "aWHALE");
  assert_eq!(
    res,
    Response::new()
      .add_attributes(vec![
        ("action", "asset/claim_rewards"),
        ("user", "user1"),
        ("assets", "native:aWHALE"),
        ("reward_amount", "100000"),
      ])
      .add_message(CosmosMsg::Bank(BankMsg::Send {
        to_address: "user1".to_string(),
        amount: coins(100000, "uluna"),
      }))
  );

  // Claiming again should get 0 rewards
  let res = claim_rewards(deps.as_mut(), "user1", "aWHALE");
  assert_eq!(
    res,
    Response::new().add_attributes(vec![
      ("action", "asset/claim_rewards"),
      ("user", "user1"),
      ("assets", "native:aWHALE"),
      ("reward_amount", "0"),
    ])
  );
}

#[test]
fn claim_rewards_after_staking_and_unstaking() {
  let mut deps = mock_dependencies();
  deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(2000000, "uluna")]);
  setup_contract(deps.as_mut());
  whitelist_assets(
    deps.as_mut(),
    vec![
      AssetInfo::Native("aWHALE".to_string()).into(),
      AssetInfo::Native("bWHALE".to_string()).into(),
    ],
  );
  stake(deps.as_mut(), "user1", 1000000, "aWHALE");
  stake(deps.as_mut(), "user2", 4000000, "aWHALE");
  stake(deps.as_mut(), "user2", 1000000, "bWHALE");

  ASSET_REWARD_DISTRIBUTION
    .save(
      deps.as_mut().storage,
      &vec![
        AssetDistribution {
          asset: AssetInfo::Native("aWHALE".to_string()),
          distribution: Decimal::percent(50),
          total_vp: Uint128::zero(),
        },
        AssetDistribution {
          asset: AssetInfo::Native("bWHALE".to_string()),
          distribution: Decimal::percent(50),
          total_vp: Uint128::zero(),
        },
      ],
    )
    .unwrap();

  execute(
    deps.as_mut(),
    mock_env(),
    mock_info(MOCK_CONTRACT_ADDR, &[]),
    ExecuteMsg::Callback(CallbackMsg::UpdateRewards {
      initial_balance: AssetInfo::native("uluna").with_balance(Uint128::new(1000000)),
    }),
  )
  .unwrap();
  claim_rewards(deps.as_mut(), "user1", "aWHALE");

  // Get asset reward rate
  let prev_rate = ASSET_REWARD_RATE
    .load(deps.as_mut().storage, &AssetInfo::Native("aWHALE".to_string()))
    .unwrap();

  // Unstake
  unstake(deps.as_mut(), "user1", 1000000, "aWHALE");

  // Accrue rewards again
  execute(
    deps.as_mut(),
    mock_env(),
    mock_info(MOCK_CONTRACT_ADDR, &[]),
    ExecuteMsg::Callback(CallbackMsg::UpdateRewards {
      initial_balance: AssetInfo::native("uluna").with_balance(Uint128::new(1000000)),
    }),
  )
  .unwrap();

  let curr_rate = ASSET_REWARD_RATE
    .load(deps.as_mut().storage, &AssetInfo::Native("aWHALE".to_string()))
    .unwrap();
  assert!(curr_rate > prev_rate);

  // User 1 stakes back
  stake(deps.as_mut(), "user1", 1000000, "aWHALE");

  // User 1 should not have any rewards
  let rewards = query_rewards(deps.as_ref(), "user1", "aWHALE");
  assert_eq!(rewards.reward_asset.amount, Uint128::zero());

  // User 2 should receive all the rewards in the contract
  let rewards = query_rewards(deps.as_ref(), "user2", "aWHALE");
  assert_eq!(rewards.reward_asset.amount, Uint128::new(900000));
  let rewards = query_rewards(deps.as_ref(), "user2", "bWHALE");
  assert_eq!(rewards.reward_asset.amount, Uint128::new(1000000));
}

#[test]
fn claim_rewards_after_rebalancing_emissions() {
  let mut deps = mock_dependencies();
  deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(2000000, "uluna")]);
  setup_contract(deps.as_mut());
  whitelist_assets(
    deps.as_mut(),
    vec![
      AssetInfo::Native("aWHALE".to_string()).into(),
      AssetInfo::Native("bWHALE".to_string()).into(),
    ],
  );
  stake(deps.as_mut(), "user1", 1000000, "aWHALE");
  stake(deps.as_mut(), "user2", 1000000, "bWHALE");

  ASSET_REWARD_DISTRIBUTION
    .save(
      deps.as_mut().storage,
      &vec![
        AssetDistribution {
          asset: AssetInfo::Native("aWHALE".to_string()),
          distribution: Decimal::percent(50),
          total_vp: Uint128::zero(),
        },
        AssetDistribution {
          asset: AssetInfo::Native("bWHALE".to_string()),
          distribution: Decimal::percent(50),
          total_vp: Uint128::zero(),
        },
      ],
    )
    .unwrap();

  execute(
    deps.as_mut(),
    mock_env(),
    mock_info(MOCK_CONTRACT_ADDR, &[]),
    ExecuteMsg::Callback(CallbackMsg::UpdateRewards {
      initial_balance: AssetInfo::native("uluna").with_balance(Uint128::new(1000000)),
    }),
  )
  .unwrap();

  ASSET_REWARD_DISTRIBUTION
    .save(
      deps.as_mut().storage,
      &vec![
        AssetDistribution {
          asset: AssetInfo::Native("aWHALE".to_string()),
          distribution: Decimal::percent(100),
          total_vp: Uint128::zero(),
        },
        AssetDistribution {
          asset: AssetInfo::Native("bWHALE".to_string()),
          distribution: Decimal::percent(0),
          total_vp: Uint128::zero(),
        },
      ],
    )
    .unwrap();

  execute(
    deps.as_mut(),
    mock_env(),
    mock_info(MOCK_CONTRACT_ADDR, &[]),
    ExecuteMsg::Callback(CallbackMsg::UpdateRewards {
      initial_balance: AssetInfo::native("uluna").with_balance(Uint128::new(1000000)),
    }),
  )
  .unwrap();

  let rewards = query_rewards(deps.as_ref(), "user1", "aWHALE");
  assert_eq!(rewards.reward_asset.amount, Uint128::new(1500000));
  // User 2 should receive all the rewards in the contract
  let rewards = query_rewards(deps.as_ref(), "user2", "bWHALE");
  assert_eq!(rewards.reward_asset.amount, Uint128::new(500000));
}

#[test]
fn test_set_asset_reward_distribution() {
  let mut deps = mock_dependencies();
  deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(2000000, "uluna")]);
  setup_contract(deps.as_mut());

  let err = execute(
    deps.as_mut(),
    mock_env(),
    mock_info("unauthorized", &[]),
    ExecuteMsg::SetAssetRewardDistribution(asset_distribution_1()),
  )
  .unwrap_err();

  // only the governance can set the asset reward distribution
  assert_eq!(
    err,
    SharedError::UnauthorizedMissingRight("ASSET_GAUGE".to_string(), "unauthorized".to_string())
      .into()
  );

  execute(
    deps.as_mut(),
    mock_env(),
    mock_info("gov", &[]),
    ExecuteMsg::SetAssetRewardDistribution(asset_distribution_1()),
  )
  .unwrap();

  let reward_distribution = query_asset_reward_distribution(deps.as_ref());
  assert_eq!(reward_distribution, asset_distribution_1());

  execute(
    deps.as_mut(),
    mock_env(),
    mock_info("gov", &[]),
    ExecuteMsg::SetAssetRewardDistribution(asset_distribution_2()),
  )
  .unwrap();

  let reward_distribution = query_asset_reward_distribution(deps.as_ref());
  assert_eq!(reward_distribution, asset_distribution_2());

  execute(
    deps.as_mut(),
    mock_env(),
    mock_info("gov", &[]),
    ExecuteMsg::SetAssetRewardDistribution(asset_distribution_1()),
  )
  .unwrap();

  let reward_distribution = query_asset_reward_distribution(deps.as_ref());
  assert_eq!(reward_distribution, asset_distribution_1());

  let err = execute(
    deps.as_mut(),
    mock_env(),
    mock_info("gov", &[]),
    ExecuteMsg::SetAssetRewardDistribution(asset_distribution_broken_1()),
  )
  .unwrap_err();

  assert_eq!(err, ContractError::InvalidDistribution {});

  let err = execute(
    deps.as_mut(),
    mock_env(),
    mock_info("gov", &[]),
    ExecuteMsg::SetAssetRewardDistribution(asset_distribution_broken_2()),
  )
  .unwrap_err();

  assert_eq!(err, ContractError::InvalidDistribution {});
}
