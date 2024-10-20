use crate::{
  common::{
    helpers::{native, u, Cw20, Uint128},
    suite::TestingSuite,
  },
  extensions::app_response_ext::{EventChecker, Valid},
};
use cosmwasm_std::{attr, Decimal, StdError};
use cw721::{AllNftInfoResponse, NftInfoResponse, OwnerOfResponse, TokensResponse};
use cw_asset::AssetInfoBase;
use ve3_asset_gauge::error::ContractError;
use ve3_shared::{
  constants::{MAX_LOCK_PERIODS, SECONDS_PER_WEEK},
  error::SharedError,
  extensions::decimal_ext::DecimalExt,
  helpers::{
    slope::adjust_vp_and_slope,
    time::{Time, Times},
  },
  msgs_asset_gauge::*,
  msgs_asset_staking::AssetDistribution,
  msgs_voting_escrow::{Extension, Trait, VotingPowerResponse},
};

#[test]
fn test_total_vp() {
  let mut suite = TestingSuite::def();
  suite.init();
  let user1 = suite.address("user1").to_string();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, native("uluna", 1000u128), "user1", |res| {
      res.assert_valid()
    })
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, native("uluna", 1000u128), "user2", |res| {
      res.assert_valid()
    })
    .q_ve_all_tokens(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        TokensResponse {
          tokens: vec!["1".to_string(), "2".to_string()]
        }
      )
    })
    .q_ve_all_nft_info("1".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        AllNftInfoResponse::<Extension> {
          access: OwnerOfResponse {
            owner: user1.clone(),
            approvals: vec![]
          },
          info: NftInfoResponse {
            token_uri: None,
            extension: Extension {
              image: None,
              description: None,
              name: None,
              attributes: Some(vec![
                Trait {
                  display_type: None,
                  trait_type: "asset".to_string(),
                  value: "native:uluna:1000".to_string()
                },
                Trait {
                  display_type: None,
                  trait_type: "start".to_string(),
                  value: "74".to_string()
                },
                Trait {
                  display_type: None,
                  trait_type: "end".to_string(),
                  value: "76".to_string()
                }
              ])
            }
          }
        }
      );
    })
    .q_ve_total_vamp(None, |res| {
      let mut vp =
        Decimal::from_ratio(90_u64 * 2, MAX_LOCK_PERIODS * 10).checked_mul_uint(u(1000)).unwrap();
      adjust_vp_and_slope(&mut vp, 2).unwrap();
      let total_vp = u(1000) + vp;

      assert_eq!(
        res.unwrap(),
        VotingPowerResponse {
          vp: total_vp * u(2),
          fixed: u(2000),
          voting_power: u(344)
        }
      )
    });
}

#[test]
fn test_locks_transfer() {
  let mut suite = TestingSuite::def();
  suite.init();
  let user2 = suite.address("user2").to_string();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, native("uluna", 1000u128), "user1", |res| {
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("token_id", "1"));
    })
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, native("uluna", 1000u128), "user2", |res| {
      res.assert_attribute(attr("token_id", "2"));
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(172),
          fixed_amount: u(1000),
          slope: u(86),
          gauge_votes: vec![]
        }
      );
    })
    .e_ve_transfer_nft(user2.clone(), "1".to_string(), "user1", |res| {
      res.assert_attribute(attr("new_owner", user2.clone()));
    })
    .q_ve_total_vamp(None, |res| {
      let mut vp =
        Decimal::from_ratio(90_u64 * 2, MAX_LOCK_PERIODS * 10).checked_mul_uint(u(1000)).unwrap();
      adjust_vp_and_slope(&mut vp, 2).unwrap();
      let total_vp = u(1000) + vp;

      assert_eq!(total_vp, u(1172));

      assert_eq!(
        res.unwrap(),
        VotingPowerResponse {
          vp: total_vp * u(2),
          fixed: u(2000),
          voting_power: u(344)
        }
      )
    })
    .q_ve_owner_of("1".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        OwnerOfResponse {
          owner: user2.to_string(),
          approvals: vec![]
        }
      )
    })
    .q_ve_owner_of("2".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        OwnerOfResponse {
          owner: user2.to_string(),
          approvals: vec![]
        }
      )
    })
    .q_gauge_user_info("user2", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(172 * 2),
          fixed_amount: u(1000 * 2),
          slope: u(86 * 2),
          gauge_votes: vec![]
        }
      );
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(0),
          fixed_amount: u(0),
          slope: u(0),
          gauge_votes: vec![]
        }
      );
    })
    .add_one_period()
    .q_gauge_user_info("user2", Some(Time::Current), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(172 * 2),
          fixed_amount: u(1000 * 2),
          slope: u(86 * 2),
          gauge_votes: vec![]
        }
      );
    })
    .q_gauge_user_info("user1", Some(Time::Current), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(0),
          fixed_amount: u(0),
          slope: u(0),
          gauge_votes: vec![]
        }
      );
    })
    .q_gauge_user_info("user2", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(86 * 2),
          fixed_amount: u(1000 * 2),
          slope: u(86 * 2),
          gauge_votes: vec![]
        }
      );
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(0),
          fixed_amount: u(0),
          slope: u(0),
          gauge_votes: vec![]
        }
      );
    })
    .add_one_period()
    .q_gauge_user_info("user2", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(0),
          fixed_amount: u(1000 * 2),
          slope: u(0),
          gauge_votes: vec![]
        }
      );
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(0),
          fixed_amount: u(0),
          slope: u(0),
          gauge_votes: vec![]
        }
      );
    });
}

#[test]
fn test_vote_asserts() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();
  let allowed_cw20 = addr.lp_cw20.to_string();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, native("uluna", 1000u128), "user1", |res| {
      res.unwrap();
    })
    .init_def_staking_whitelist()
    .use_staking_2()
    .init_def_staking_whitelist()
    .use_staking_1()
    .e_gauge_vote(addr.gauge_1.clone(), vec![], "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::ZeroVotingPower(addr.user2.to_string(), 75));
    })
    .e_gauge_vote(
      addr.gauge_1.clone(),
      vec![("native:uluna".to_string(), 10000), ("native:uluna".to_string(), 10000)],
      "user1",
      |res| {
        let res = res.unwrap_err().downcast::<ContractError>().unwrap();
        assert_eq!(res, ContractError::InvalidAsset("native:uluna".to_string()));
      },
    )
    .e_gauge_vote(
      addr.gauge_1.clone(),
      vec![(addr.lp_native_info_str(), 10000), (addr.lp_native_info_str(), 10000)],
      "user1",
      |res| {
        let res = res.unwrap_err().downcast::<ContractError>().unwrap();
        assert_eq!(res, ContractError::DuplicatedVotes {});
      },
    )
    .e_gauge_vote(
      addr.gauge_1.clone(),
      vec![(addr.lp_native_info_str(), 10000), (format!("cw20:{allowed_cw20}"), 10000)],
      "user1",
      |res| {
        let res = res.unwrap_err().downcast::<ContractError>().unwrap();
        assert_eq!(
          res,
          ContractError::Std(StdError::generic_err("Basic points sum exceeds limit"))
        );
      },
    )
    .e_gauge_vote(
      addr.gauge_1.clone(),
      vec![(addr.lp_native_info_str(), 5000), (format!("cw20:{allowed_cw20}"), 5000)],
      "user2",
      |res| {
        let res = res.unwrap_err().downcast::<ContractError>().unwrap();
        assert_eq!(res, ContractError::ZeroVotingPower(addr.user2.to_string(), 75));
      },
    )
    .e_gauge_vote(
      addr.gauge_1.clone(),
      vec![(addr.lp_native_info_str(), 5000), (format!("cw20:{allowed_cw20}"), 5000)],
      "user1",
      |res| {
        res.assert_attribute(attr("action", "gauge/vote"));
        res.assert_attribute(attr("vp", "1172"));
      },
    );
}

#[test]
fn test_query_infos() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();
  let allowed_cw20 = addr.lp_cw20.to_string();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, native("uluna", 1000u128), "user1", |res| {
      res.assert_valid()
    })
    .init_def_staking_whitelist()
    .e_gauge_vote(
      addr.gauge_1.clone(),
      vec![(addr.lp_native_info_str(), 5000), (format!("cw20:{allowed_cw20}"), 5000)],
      "user1",
      |res| {
        res.assert_valid()
      },
    )
    .q_gauge_gauge_infos(addr.gauge_1.clone(), None, None, |res| {
      // current period still empty
      assert_eq!(
        res.unwrap(),
        vec![
          (
            format!("cw20:{allowed_cw20}"),
            VotedInfoResponse {
              voting_power: u(0),
              fixed_amount: u(0),
              slope: u(0)
            }
          ),
          (
            addr.lp_native_info_str(),
            VotedInfoResponse {
              voting_power: u(0),
              fixed_amount: u(0),
              slope: u(0)
            }
          )
        ]
      );
    })
    .q_gauge_gauge_infos(addr.gauge_1.clone(), None, Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          (
            format!("cw20:{allowed_cw20}"),
            VotedInfoResponse {
              voting_power: u(86),
              fixed_amount: u(500),
              slope: u(43)
            }
          ),
          (
            addr.lp_native_info_str(),
            VotedInfoResponse {
              voting_power: u(86),
              fixed_amount: u(500),
              slope: u(43)
            }
          )
        ]
      );
    })
    .add_one_period()
    .q_gauge_gauge_infos(addr.gauge_1.clone(), None, Some(Time::Current), |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          (
            format!("cw20:{allowed_cw20}"),
            VotedInfoResponse {
              voting_power: u(86),
              fixed_amount: u(500),
              slope: u(43)
            }
          ),
          (
            addr.lp_native_info_str(),
            VotedInfoResponse {
              voting_power: u(86),
              fixed_amount: u(500),
              slope: u(43)
            }
          )
        ]
      );
    })
    .q_gauge_gauge_info(addr.gauge_1.clone(), addr.lp_native_info_str(), None, |res| {
      assert_eq!(
        res.unwrap(),
        VotedInfoResponse {
          voting_power: u(86),
          fixed_amount: u(500),
          slope: u(43)
        }
      );
    })
    .q_gauge_user_first_participation("user1", |res| {
      assert_eq!(
        res.unwrap(),
        UserFirstParticipationResponse {
          period: Some(75)
        }
      );
    })
    .q_gauge_user_first_participation("user2", |res| {
      assert_eq!(
        res.unwrap(),
        UserFirstParticipationResponse {
          period: None
        }
      );
    })
    .q_gauge_user_shares("user2", None, |res| {
      let res = res.unwrap_err();
      assert_eq!(res.to_string(), "Generic error: Querier contract error: User 'terra1vqjarrly327529599rcc4qhzvhwe34pp5uyy4gylvxe5zupeqx3sl7x356' has no voting power in period 75".to_string());
    })
    .q_gauge_user_shares("user1", None, |res| {
      let res = res.unwrap_err();
      assert_eq!(res.to_string(), "Generic error: Querier contract error: Gauge distribution not yet executed. period 75".to_string());
    })
    .e_gauge_set_distribution("user1", |res| {
      res.unwrap();
    })
    .q_gauge_user_shares("user1", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserSharesResponse {
          shares: vec![UserShare {
            gauge: addr.gauge_1.clone(),
            asset: addr.lp_native_info_checked(),
            period: 75,
            user_vp: u(586),
            total_vp: u(586)
          },
          UserShare {
            gauge: addr.gauge_1.clone(),
            asset: AssetInfoBase::Cw20(addr.lp_cw20.clone()),
            period: 75,
            user_vp: u(586),
            total_vp: u(586)
          }]
        }
      );
    })
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 20, native("uluna", 4000u128), "user1", |res| {
      res.unwrap();
    })
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 20, native("uluna", 10000u128), "user2", |res| {
      res.unwrap();
    })
    .e_gauge_vote(
      addr.gauge_1.clone(),
      vec![(addr.lp_native_info_str(), 10000)],
      "user1",
      |res| {
        res.unwrap();
      },
    );

  suite
    .add_periods(4)
    .e_gauge_vote(
      addr.gauge_1.clone(),
      vec![(addr.lp_native_info_str(), 8000), (format!("cw20:{allowed_cw20}"), 2000)],
      "user2",
      |res| {
        res.unwrap();
      },
    )
    .add_periods(8)
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .q_staking_reward_distribution(|res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        vec![
          AssetDistribution {
            asset: addr.lp_native_info_checked(),
            total_vp: u(22342),
            distribution: Decimal::one() - Decimal::from_ratio(u(3557), u(22342 + 3557)),
          },
          AssetDistribution {
            asset: Cw20(addr.lp_cw20.clone()),
            distribution: Decimal::from_ratio(u(3557), u(22342 + 3557)),
            total_vp: u(3557)
          }
        ]
      )
    })
    .q_gauge_user_shares("user1", Some(Times::Periods((75..87).collect())), |res| {
      assert_eq!(
        res.unwrap(),
        UserSharesResponse {
          shares: vec![
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 75,
              user_vp: Uint128(586),
              total_vp: Uint128(586)
            },
            UserShare {
              gauge: "stable".into(),
              asset: Cw20(addr.lp_cw20.clone()),
              period: 75,
              user_vp: Uint128(586),
              total_vp: Uint128(586)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 76,
              user_vp: Uint128(12006),
              total_vp: Uint128(12006)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 77,
              user_vp: Uint128(11574),
              total_vp: Uint128(11574)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 78,
              user_vp: Uint128(11228),
              total_vp: Uint128(11228)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 79,
              user_vp: Uint128(10882),
              total_vp: Uint128(10882)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 80,
              user_vp: Uint128(10536),
              total_vp: Uint128(29608)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 81,
              user_vp: Uint128(10190),
              total_vp: Uint128(28570)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 82,
              user_vp: Uint128(9844),
              total_vp: Uint128(27532)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 83,
              user_vp: Uint128(9498),
              total_vp: Uint128(26494)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 84,
              user_vp: Uint128(9152),
              total_vp: Uint128(25456)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 85,
              user_vp: Uint128(8806),
              total_vp: Uint128(24418)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 86,
              user_vp: Uint128(8460),
              total_vp: Uint128(23380)
            }
          ]
        }
      );
    })
    .q_gauge_user_shares("user2", Some(Times::Periods((76..87).collect())), |res| {
      assert_eq!(
        res.unwrap(),
        UserSharesResponse {
          shares: vec![
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 80,
              // cross check is possible by checking in the one before that the sum is correct.
              // between this and the stable-lp-80 from above
              // example
              // 29608 = 19072+10536
              user_vp: Uint128(19072),
              total_vp: Uint128(29608)
            },
            UserShare {
              gauge: "stable".into(),
              asset: Cw20(addr.lp_cw20.clone()),
              period: 80,
              user_vp: Uint128(4768),
              total_vp: Uint128(4768)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 81,
              // 28570 = 18380+10190...
              user_vp: Uint128(18380),
              total_vp: Uint128(28570)
            },
            UserShare {
              gauge: "stable".into(),
              asset: Cw20(addr.lp_cw20.clone()),
              period: 81,
              user_vp: Uint128(4595),
              total_vp: Uint128(4595)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 82,
              user_vp: Uint128(17688),
              total_vp: Uint128(27532)
            },
            UserShare {
              gauge: "stable".into(),
              asset: Cw20(addr.lp_cw20.clone()),
              period: 82,
              user_vp: Uint128(4422),
              total_vp: Uint128(4422)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 83,
              user_vp: Uint128(16996),
              total_vp: Uint128(26494)
            },
            UserShare {
              gauge: "stable".into(),
              asset: Cw20(addr.lp_cw20.clone()),
              period: 83,
              user_vp: Uint128(4249),
              total_vp: Uint128(4249)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 84,
              user_vp: Uint128(16304),
              total_vp: Uint128(25456)
            },
            UserShare {
              gauge: "stable".into(),
              asset: Cw20(addr.lp_cw20.clone()),
              period: 84,
              user_vp: Uint128(4076),
              total_vp: Uint128(4076)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 85,
              user_vp: Uint128(15612),
              total_vp: Uint128(24418)
            },
            UserShare {
              gauge: "stable".into(),
              asset: Cw20(addr.lp_cw20.clone()),
              period: 85,
              user_vp: Uint128(3903),
              total_vp: Uint128(3903)
            },
            UserShare {
              gauge: "stable".into(),
              asset: addr.lp_native_info_checked(),
              period: 86,
              user_vp: Uint128(14920),
              total_vp: Uint128(23380)
            },
            UserShare {
              gauge: "stable".into(),
              asset: Cw20(addr.lp_cw20.clone()),
              period: 86,
              user_vp: Uint128(3730),
              total_vp: Uint128(3730)
            }
          ]
        }
      );
    });
}

#[test]
fn test_config() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .q_gauge_config(|res| {
      assert_eq!(
        res.unwrap(),
        Config {
          global_config_addr: addr.ve3_global_config.clone(),
          gauges: vec![
            GaugeConfig {
              name: addr.gauge_1.clone(),
              min_gauge_percentage: Decimal::percent(10)
            },
            GaugeConfig {
              name: addr.gauge_2.clone(),
              min_gauge_percentage: Decimal::percent(0)
            },
            GaugeConfig {
              name: addr.gauge_3.clone(),
              min_gauge_percentage: Decimal::percent(0)
            },
          ],
          rebase_asset: addr.ampluna_info_checked()
        }
      )
    })
    .e_gauge_update_config(
      Some(GaugeConfig {
        name: "any".to_string(),
        min_gauge_percentage: Decimal::percent(1),
      }),
      None,
      "anyone",
      |res| res.assert_error(ContractError::SharedError(SharedError::Unauthorized {})),
    )
    .e_gauge_update_config(
      Some(GaugeConfig {
        name: "any".to_string(),
        min_gauge_percentage: Decimal::percent(21),
      }),
      None,
      "creator",
      |res| {
        res.assert_error(ContractError::SharedError(SharedError::NotSupported(
          "min_gauge_percentage needs to be less than 20%".to_string(),
        )))
      },
    )
    .e_gauge_update_config(
      Some(GaugeConfig {
        name: "any".to_string(),
        min_gauge_percentage: Decimal::percent(1),
      }),
      Some(addr.gauge_1.to_string()),
      "creator",
      |res| res.assert_valid(),
    )
    .q_gauge_config(|res| {
      assert_eq!(
        res.unwrap(),
        Config {
          global_config_addr: addr.ve3_global_config.clone(),
          gauges: vec![
            GaugeConfig {
              name: addr.gauge_2.clone(),
              min_gauge_percentage: Decimal::percent(0)
            },
            GaugeConfig {
              name: addr.gauge_3.to_string(),
              min_gauge_percentage: Decimal::percent(0)
            },
            GaugeConfig {
              name: "any".to_string(),
              min_gauge_percentage: Decimal::percent(1)
            },
          ],
          rebase_asset: addr.ampluna_info_checked()
        }
      )
    });
}

#[test]
fn test_user_infos() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user1", |res| {
      res.assert_valid()
    })
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.ampluna(1000), "user2", |res| {
      res.assert_valid()
    })
    .q_gauge_user_infos(None, None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          (
            addr.user1.clone(),
            VotedInfoResponse {
              voting_power: u(0),
              fixed_amount: u(0),
              slope: u(0)
            }
          ),
          (
            addr.user2.clone(),
            VotedInfoResponse {
              voting_power: u(0),
              fixed_amount: u(0),
              slope: u(0)
            }
          )
        ]
      )
    })
    // Next = 75
    .q_gauge_user_infos(None, None, Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          (
            addr.user1.clone(),
            VotedInfoResponse {
              voting_power: u(172),
              fixed_amount: u(1000),
              slope: u(86)
            }
          ),
          (
            addr.user2.clone(),
            VotedInfoResponse {
              voting_power: u(206),
              fixed_amount: u(1200),
              slope: u(103)
            }
          )
        ]
      )
    })
    .q_gauge_user_infos(None, None, Some(Time::Period(76)), |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          (
            addr.user1.clone(),
            VotedInfoResponse {
              voting_power: u(86),
              fixed_amount: u(1000),
              slope: u(86)
            }
          ),
          (
            addr.user2.clone(),
            VotedInfoResponse {
              voting_power: u(103),
              fixed_amount: u(1200),
              slope: u(103)
            }
          )
        ]
      )
    })
    .q_gauge_user_infos(None, Some(1), Some(Time::Period(76)), |res| {
      assert_eq!(
        res.unwrap(),
        vec![(
          addr.user1.clone(),
          VotedInfoResponse {
            voting_power: u(86),
            fixed_amount: u(1000),
            slope: u(86)
          }
        )]
      )
    })
    .q_gauge_user_infos(Some(addr.user1.to_string()), None, Some(Time::Period(76)), |res| {
      assert_eq!(
        res.unwrap(),
        vec![(
          addr.user2.clone(),
          VotedInfoResponse {
            voting_power: u(103),
            fixed_amount: u(1200),
            slope: u(103)
          }
        )]
      )
    });
}
