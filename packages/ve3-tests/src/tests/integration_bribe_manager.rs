use crate::{
  common::{helpers::u, suite::TestingSuite},
  extensions::app_response_ext::{EventChecker, Valid},
};
use cosmwasm_std::attr;
use ve3_bribe_manager::error::ContractError;
use ve3_shared::{
  constants::WEEK,
  error::SharedError,
  helpers::{assets::Assets, time::Time},
  msgs_bribe_manager::{
    BribeBucket, BribeBuckets, Config, NextClaimPeriodResponse, UserClaimableResponse,
  },
};

#[test]
fn test_lock_add_bribes() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(1000), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(2000), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .def_gauge_1_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_1_vote(7500, 2500, "user2", |res| res.assert_valid())
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .e_bribe_add_bribe_native(
      addr.uluna(1000),
      &addr.gauge_1,
      addr.lp_cw20_info(),
      ve3_shared::msgs_bribe_manager::BribeDistribution::Next,
      None,
      // creator part of AT_FREE_BRIBES
      "creator",
      |res| {
        res.assert_attribute(attr("action", "bribe/add_bribe"));
      },
    )
    .e_bribe_add_bribe_native(
      addr.uluna(1000),
      &addr.gauge_1,
      addr.lp_cw20_info(),
      ve3_shared::msgs_bribe_manager::BribeDistribution::Next,
      Some(addr.uluna(10000000)),
      "user1",
      |res| {
        res.assert_attribute(attr("action", "bribe/add_bribe"));
        res.assert_attribute(attr("start", "76"));
        res.assert_attribute_ty("transfer", attr("recipient", addr.fee_recipient.to_string()));
        res.assert_attribute_ty("transfer", attr("amount", "10000000uluna"));
      },
    )
    .e_bribe_add_bribe_native(
      addr.uluna(1000000),
      &addr.gauge_1,
      addr.lp_native_info(),
      ve3_shared::msgs_bribe_manager::BribeDistribution::Func {
        start: Some(80),
        end: 89,
        func_type: ve3_shared::msgs_bribe_manager::FuncType::Linear,
      },
      Some(addr.uluna(10000000)),
      "user2",
      |res| {
        res.assert_attribute(attr("action", "bribe/add_bribe"));
      },
    )
    .e_bribe_add_bribe_cw20(
      addr.ampluna(1000000),
      &addr.gauge_1,
      addr.lp_native_info(),
      ve3_shared::msgs_bribe_manager::BribeDistribution::Func {
        start: Some(80),
        end: 179,
        func_type: ve3_shared::msgs_bribe_manager::FuncType::Linear,
      },
      Some(addr.uluna(10000000)),
      "user2",
      |res| {
        res.assert_attribute(attr("action", "bribe/add_bribe"));
        res.assert_attribute(attr("start", "80"));
        res.assert_attribute(attr("end", "179"));
      },
    )
    .e_bribe_withdraw_bribes(80, "user2", |res| {
      // withdraw both for 80
      res.assert_attribute(attr("action", "bribe/withdraw_bribes"));
      res.assert_attribute_ty("transfer", attr("recipient", addr.user2.to_string()));
      res.assert_attribute_ty("transfer", attr("amount", "100000uluna"));
      res.assert_attribute(attr("to", addr.user2.to_string()));
      res.assert_attribute(attr("amount", "10000"));
    })
    .add_one_period()
    // block period = 76
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    // user1 has twice as much VP than user2, but half the amount of tokens -> same amount received (both 50%)
    .q_bribe_user_claimable("user1", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserClaimableResponse {
          start: 75,
          end: 76,
          buckets: vec![BribeBucket {
            gauge: "stable".to_string(),
            asset: Some(addr.lp_cw20_info_checked()),
            assets: Assets(vec![addr.uluna(1000)])
          }]
        }
      )
    })
    .e_bribe_claim_bribes(None, "user1", |res| {
      res.assert_attribute(attr("action", "bribe/claim_bribes"));
      res.assert_attribute_ty("transfer", attr("recipient", addr.user1.to_string()));
      res.assert_attribute_ty("transfer", attr("amount", "1000uluna"));
      res.assert_attribute(attr("periods", "75,76"));
    })
    .q_bribe_user_claimable("user1", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserClaimableResponse {
          start: 0,
          end: 0,
          buckets: vec![]
        }
      )
    })
    .q_bribe_user_claimable("user2", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserClaimableResponse {
          start: 75,
          end: 76,
          buckets: vec![BribeBucket {
            gauge: "stable".to_string(),
            asset: Some(addr.lp_cw20_info_checked()),
            assets: Assets(vec![addr.uluna(1000)])
          }]
        }
      )
    })
    .add_periods(4)
    // block period = 80
    // 80 still empty, as it has been withdrawn
    .e_gauge_set_distribution("creator", |res| res.assert_valid())
    .q_bribe_user_claimable("user1", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserClaimableResponse {
          start: 77,
          end: 80,
          buckets: vec![]
        }
      )
    })
    .add_one_period()
    // 81 has first thing
    .e_gauge_set_distribution("creator", |res| res.assert_valid())
    .q_bribe_user_claimable("user1", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserClaimableResponse {
          start: 77,
          end: 81,
          buckets: vec![BribeBucket {
            gauge: "stable".to_string(),
            asset: Some(addr.lp_native_info_checked()),
            assets: Assets(vec![addr.uluna(24987), addr.ampluna(2498)])
          }]
        }
      )
    })
    .q_bribe_user_claimable("user2", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserClaimableResponse {
          start: 75,
          end: 81,
          buckets: vec![
            // from before
            BribeBucket {
              gauge: "stable".to_string(),
              asset: Some(addr.lp_cw20_info_checked()),
              assets: Assets(vec![addr.uluna(1000)])
            },
            BribeBucket {
              gauge: "stable".to_string(),
              asset: Some(addr.lp_native_info_checked()),
              // on native lp it is 75% for user2, 25% for user1
              assets: Assets(vec![addr.uluna(74962), addr.ampluna(7496)])
            }
          ]
        }
      )
    })
    .e_bribe_claim_bribes(Some(vec![75, 76]), "user1", |res| {
      res.assert_error(ContractError::BribeAlreadyClaimed(76))
    })
    .q_bribe_next_claim_period("user1", |res| {
      assert_eq!(
        res.unwrap(),
        NextClaimPeriodResponse {
          period: 77
        }
      )
    })
    .e_bribe_claim_bribes(Some(vec![77, 78, 79, 80, 81]), "user1", |res| {
      res.assert_attribute(attr("action", "bribe/claim_bribes"));
      res.assert_attribute_ty("transfer", attr("recipient", addr.user1.to_string()));
      res.assert_attribute_ty("transfer", attr("amount", "24987uluna"));
      res.assert_attribute(attr("to", addr.user1.to_string()));
      res.assert_attribute(attr("amount", "2498"));
    })
    .q_bribe_user_claimable("user1", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserClaimableResponse {
          start: 0,
          end: 0,
          buckets: vec![]
        }
      )
    })
    .q_bribe_bribes(Some(Time::Period(76)), |res| {
      assert_eq!(
        res.unwrap(),
        BribeBuckets {
          buckets: vec![BribeBucket {
            gauge: addr.gauge_1.to_string(),
            asset: Some(addr.lp_cw20_info_checked()),
            assets: Assets(vec![addr.uluna(2000)])
          }]
        }
      )
    })
    .q_bribe_bribes(Some(Time::Period(89)), |res| {
      assert_eq!(
        res.unwrap(),
        BribeBuckets {
          buckets: vec![BribeBucket {
            gauge: addr.gauge_1.to_string(),
            asset: Some(addr.lp_native_info_checked()),
            assets: Assets(vec![addr.uluna(100000), addr.ampluna(10000)])
          }]
        }
      )
    })
    .add_periods(8)
    // 89 deployed rewards fully for uluna, ampluna still going over 100 periods, so only 10 (8 additional)
    .e_gauge_set_distribution("anyone", |res| res.assert_valid())
    .q_bribe_user_claimable("user1", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserClaimableResponse {
          start: 82,
          end: 89,
          buckets: vec![BribeBucket {
            gauge: "stable".to_string(),
            asset: Some(addr.lp_native_info_checked()),
            assets: Assets(vec![addr.uluna(24987 * 8), addr.ampluna(2498 * 8)])
          }]
        }
      )
    })
    // user 2 still has 9 periods to claim from second batch of incentives
    .q_bribe_user_claimable("user2", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserClaimableResponse {
          start: 75,
          end: 89,
          buckets: vec![
            // from before
            BribeBucket {
              gauge: "stable".to_string(),
              asset: Some(addr.lp_cw20_info_checked()),
              assets: Assets(vec![addr.uluna(1000)])
            },
            BribeBucket {
              gauge: "stable".to_string(),
              asset: Some(addr.lp_native_info_checked()),
              // on native lp it is 75% for user2, 25% for user1
              assets: Assets(vec![addr.uluna(74962 * 9), addr.ampluna(7496 * 9)])
            }
          ]
        }
      )
    })
    .q_bribe_next_claim_period("user2", |res| {
      assert_eq!(
        res.unwrap(),
        NextClaimPeriodResponse {
          period: 75
        }
      )
    })
    .q_bribe_next_claim_period("creator", |res| {
      assert_eq!(
        res.unwrap(),
        NextClaimPeriodResponse {
          period: 89
        }
      )
    })
    .e_bribe_claim_bribes(None, "user2", |res| {
      res.assert_attribute(attr("action", "bribe/claim_bribes"));
      res.assert_attribute_ty("transfer", attr("recipient", addr.user2.to_string()));
      res.assert_attribute_ty("transfer", attr("amount", format!("{0}uluna", 74962 * 9 + 1000)));
      res.assert_attribute(attr("to", addr.user2.to_string()));
      res.assert_attribute(attr("amount", (7496 * 9).to_string()));
    })
    .q_bribe_bribes(Some(Time::Period(89)), |res| {
      assert_eq!(
        res.unwrap(),
        BribeBuckets {
          buckets: vec![BribeBucket {
            gauge: addr.gauge_1.to_string(),
            asset: Some(addr.lp_native_info_checked()),
            assets: Assets(vec![addr.uluna(100000), addr.ampluna(10000)])
          }]
        }
      )
    });
}

#[test]
fn test_errors() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(1000), "user1", |res| res.assert_valid())
    .add_one_period()
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(2000), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .def_gauge_1_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_1_vote(7500, 2500, "user2", |res| res.assert_valid())
    // .e_bribe_add_bribe_native(bribe, gauge, for_info, distribution, sender, result)
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .e_bribe_add_bribe_native(
      addr.uluna(1000),
      &addr.gauge_1,
      addr.lp_cw20_info(),
      ve3_shared::msgs_bribe_manager::BribeDistribution::Next,
      None,
      "user1",
      |res| {
        res.assert_error(ContractError::SharedError(SharedError::WrongDeposit(
          "expected 10001000uluna coins".to_string(),
        )))
      },
    )
    .e_bribe_add_bribe_native(
      addr.fake_native(10000),
      &addr.gauge_1,
      addr.lp_native_info(),
      ve3_shared::msgs_bribe_manager::BribeDistribution::Func {
        start: Some(80),
        end: 179,
        func_type: ve3_shared::msgs_bribe_manager::FuncType::Linear,
      },
      Some(addr.uluna(10000000)),
      "user2",
      |res| res.assert_error(ContractError::AssetNotWhitelisted),
    )
    .e_bribe_add_bribe_native(
      addr.uluna(1000),
      &addr.gauge_1,
      addr.lp_cw20_info(),
      ve3_shared::msgs_bribe_manager::BribeDistribution::Specific(vec![(10u64, u(1000))]),
      None,
      // creator part of AT_FREE_BRIBES
      "creator",
      |res| {
        res.assert_error(ContractError::BribesAlreadyDistributing);
      },
    )
    .e_bribe_add_bribe_native(
      addr.uluna(1000),
      &addr.gauge_1,
      addr.lp_cw20_info(),
      ve3_shared::msgs_bribe_manager::BribeDistribution::Specific(vec![
        (100u64, u(1000)),
        (100u64, u(1000)),
      ]),
      None,
      // creator part of AT_FREE_BRIBES
      "creator",
      |res| {
        res.assert_error(ContractError::BribeDistribution("sum not equal to deposit".to_string()));
      },
    )
    .e_bribe_whitelist_assets(vec![], "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
        "BRIBE_WHITELIST_CONTROLLER".into(),
        "terra1pgzph9rze2j2xxavx4n7pdhxlkgsq7rak245x0vk7mgh3j4le6gqvw0kq8".into(),
      )))
    })
    .e_bribe_remove_assets(vec![], "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
        "BRIBE_WHITELIST_CONTROLLER".into(),
        "terra1pgzph9rze2j2xxavx4n7pdhxlkgsq7rak245x0vk7mgh3j4le6gqvw0kq8".into(),
      )))
    })
    .e_bribe_whitelist_assets(vec![], "AT_BRIBE_WHITELIST_CONTROLLER", |res| {
      res.assert_error(ContractError::RequiresAssetInfos)
    })
    .e_bribe_remove_assets(vec![], "AT_BRIBE_WHITELIST_CONTROLLER", |res| {
      res.assert_error(ContractError::RequiresAssetInfos)
    })
    .e_bribe_claim_bribes(Some(vec![1000]), "user1", |res| {
      res.assert_error(ContractError::NoPeriodsValid)
    })
    .e_bribe_update_config(None, None, "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::Unauthorized {}))
    })
    .e_bribe_withdraw_bribes(80, "user1", |res| res.assert_error(ContractError::NoBribes))
    .e_bribe_add_bribe_native(
      addr.uluna(0),
      &addr.gauge_1,
      addr.lp_cw20_info(),
      ve3_shared::msgs_bribe_manager::BribeDistribution::Next,
      None,
      // creator part of AT_FREE_BRIBES
      "creator",
      |res| {
        res.assert_error(ContractError::SharedError(SharedError::NotSupported(
          "bribes required".to_string(),
        )))
      },
    );
}

#[test]
fn test_try_withdraw_late() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(1000), "user1", |res| res.assert_valid())
    .add_one_period()
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(2000), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .def_gauge_1_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_1_vote(7500, 2500, "user2", |res| res.assert_valid())
    // .e_bribe_add_bribe_native(bribe, gauge, for_info, distribution, sender, result)
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .e_bribe_add_bribe_native(
      addr.uluna(1000),
      &addr.gauge_1,
      addr.lp_cw20_info(),
      ve3_shared::msgs_bribe_manager::BribeDistribution::Next,
      None,
      // creator part of AT_FREE_BRIBES
      "creator",
      |res| {
        res.assert_attribute(attr("action", "bribe/add_bribe"));
        res.assert_attribute(attr("start", "77"));
        res.assert_attribute(attr("end", "77"));
      },
    )
    .add_one_period()
    .e_bribe_withdraw_bribes(77, "creator", |res| {
      res.assert_error(ContractError::BribesAlreadyDistributing)
    });
}

#[test]
fn test_update_config() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();

  suite
    .e_bribe_update_config(None, None, "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::Unauthorized {}))
    })
    .q_bribe_config(|res| {
      assert_eq!(
        res.unwrap(),
        Config {
          whitelist: vec![addr.uluna_info_checked(), addr.ampluna_info_checked()],
          allow_any: false,
          fee: addr.uluna(10000000),
          global_config_addr: addr.ve3_global_config.clone()
        }
      )
    })
    .e_bribe_update_config(Some(addr.ampluna(10).into()), None, "creator", |res| {
      res.assert_error(ContractError::SharedError(SharedError::NotSupported(
        "must be native".to_string(),
      )))
    })
    .e_bribe_update_config(Some(addr.uluna(10).into()), Some(true), "creator", |res| {
      res.assert_attribute(attr("action", "bribe/update_config"));
    })
    .q_bribe_config(|res| {
      assert_eq!(
        res.unwrap(),
        Config {
          whitelist: vec![addr.uluna_info_checked(), addr.ampluna_info_checked()],
          allow_any: true,
          fee: addr.uluna(10),
          global_config_addr: addr.ve3_global_config.clone()
        }
      )
    });
}

#[test]
fn test_any_bribe() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(1000), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(1000), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .e_bribe_update_config(None, Some(true), "creator", |res| {
      res.assert_attribute(attr("action", "bribe/update_config"));
    })
    .e_bribe_add_bribe_native(
      addr.lp_native(1000),
      &addr.gauge_1,
      addr.lp_cw20_info(),
      ve3_shared::msgs_bribe_manager::BribeDistribution::Next,
      None,
      "creator",
      |res| {
        res.assert_attribute(attr("action", "bribe/add_bribe"));
      },
    )
    .q_bribe_bribes(None, |res| {
      assert_eq!(
        res.unwrap(),
        BribeBuckets {
          buckets: vec![]
        }
      )
    })
    .q_bribe_bribes(Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        BribeBuckets {
          buckets: vec![BribeBucket {
            gauge: addr.gauge_1.to_string(),
            asset: Some(addr.lp_cw20_info_checked()),
            assets: Assets(vec![addr.lp_native(1000)])
          }]
        }
      )
    })
    .q_bribe_bribes(Some(Time::Period(75)), |res| {
      assert_eq!(
        res.unwrap(),
        BribeBuckets {
          buckets: vec![BribeBucket {
            gauge: addr.gauge_1.to_string(),
            asset: Some(addr.lp_cw20_info_checked()),
            assets: Assets(vec![addr.lp_native(1000)])
          }]
        }
      )
    });
}
