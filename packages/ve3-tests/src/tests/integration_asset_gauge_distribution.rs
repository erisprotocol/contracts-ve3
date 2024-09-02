use crate::{
  common::{helpers::u, suite::TestingSuite},
  extensions::app_response_ext::Valid,
};
use cosmwasm_std::Decimal;
use ve3_shared::{
  constants::SECONDS_PER_WEEK, helpers::time::Time, msgs_asset_gauge::GaugeDistributionResponse,
  msgs_asset_staking::AssetDistribution,
};

#[test]
fn test_gauge_distribution() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite
    .e_ve_create_lock_time_any(None, addr.uluna(1000), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time_any(None, addr.uluna(2000), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .def_gauge_1_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_1_vote(7500, 2500, "user2", |res| res.assert_valid())
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .q_gauge_distribution(addr.gauge_1.clone(), None, |res| {
      assert_eq!(
        res.unwrap(),
        GaugeDistributionResponse {
          gauge: addr.gauge_1.to_string(),
          period: 75,
          total_gauge_vp: u(30000),
          assets: vec![
            AssetDistribution {
              asset: addr.lp_native_info_checked(),
              // 0.5*1000+0.75*2000 = 2
              // we are automatically fixing rounding issues by adding it to the first
              distribution: Decimal::one() - Decimal::from_ratio(1u128, 3u128),
              total_vp: u(20000)
            },
            AssetDistribution {
              asset: addr.lp_cw20_info_checked(),
              // 0.5*1000+0.25*2000 = 1
              distribution: Decimal::from_ratio(1u128, 3u128),
              total_vp: u(10000)
            }
          ]
        }
      );
    });
}

#[test]
fn test_gauge_distributions() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite
    .e_ve_create_lock_time_any(None, addr.uluna(1000), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time_any(None, addr.uluna(2000), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .use_staking_2()
    .def_staking_whitelist_recapture()
    .use_staking_1()
    .def_gauge_1_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_1_vote(7500, 2500, "user2", |res| res.assert_valid())
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .q_gauge_distributions(None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          GaugeDistributionResponse {
            gauge: addr.gauge_1.to_string(),
            period: 75,
            total_gauge_vp: u(30000),
            assets: vec![
              AssetDistribution {
                asset: addr.lp_native_info_checked(),
                // 0.5*1000+0.75*2000 = 2
                // we are automatically fixing rounding issues by adding it to the first
                distribution: Decimal::one() - Decimal::from_ratio(1u128, 3u128),
                total_vp: u(20000)
              },
              AssetDistribution {
                asset: addr.lp_cw20_info_checked(),
                // 0.5*1000+0.25*2000 = 1
                distribution: Decimal::from_ratio(1u128, 3u128),
                total_vp: u(10000)
              }
            ]
          },
          GaugeDistributionResponse {
            gauge: addr.gauge_2.to_string(),
            period: 75,
            total_gauge_vp: u(0),
            assets: vec![]
          },
          GaugeDistributionResponse {
            gauge: addr.gauge_3.to_string(),
            period: 75,
            total_gauge_vp: u(0),
            assets: vec![]
          }
        ]
      );
    })
    .def_gauge_2_vote(10000, 0, "user1", |res| res.assert_valid())
    .def_gauge_2_vote(0, 10000, "user2", |res| res.assert_valid())
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .q_gauge_distributions(Some(Time::Period(75)), |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          GaugeDistributionResponse {
            gauge: addr.gauge_1.to_string(),
            period: 75,
            total_gauge_vp: u(30000),
            assets: vec![
              AssetDistribution {
                asset: addr.lp_native_info_checked(),
                // 0.5*1000+0.75*2000 = 2
                // we are automatically fixing rounding issues by adding it to the first
                distribution: Decimal::one() - Decimal::from_ratio(1u128, 3u128),
                total_vp: u(20000)
              },
              AssetDistribution {
                asset: addr.lp_cw20_info_checked(),
                // 0.5*1000+0.25*2000 = 1
                distribution: Decimal::from_ratio(1u128, 3u128),
                total_vp: u(10000)
              }
            ]
          },
          GaugeDistributionResponse {
            gauge: addr.gauge_2.to_string(),
            period: 75,
            total_gauge_vp: u(0),
            assets: vec![]
          },
          GaugeDistributionResponse {
            gauge: addr.gauge_3.to_string(),
            period: 75,
            total_gauge_vp: u(0),
            assets: vec![]
          }
        ]
      );
    })
    .q_gauge_distributions(Some(Time::Current), |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          GaugeDistributionResponse {
            gauge: addr.gauge_1.to_string(),
            period: 76,
            total_gauge_vp: u(30000),
            assets: vec![
              AssetDistribution {
                asset: addr.lp_native_info_checked(),
                // 0.5*1000+0.75*2000 = 2
                // we are automatically fixing rounding issues by adding it to the first
                distribution: Decimal::one() - Decimal::from_ratio(1u128, 3u128),
                total_vp: u(20000)
              },
              AssetDistribution {
                asset: addr.lp_cw20_info_checked(),
                // 0.5*1000+0.25*2000 = 1
                distribution: Decimal::from_ratio(1u128, 3u128),
                total_vp: u(10000)
              }
            ]
          },
          GaugeDistributionResponse {
            gauge: addr.gauge_2.to_string(),
            period: 76,
            total_gauge_vp: u(30000),
            assets: vec![
              AssetDistribution {
                asset: addr.lp_cw20_info_checked(),
                distribution: Decimal::one() - Decimal::from_ratio(1u128, 3u128),
                total_vp: u(20000)
              },
              AssetDistribution {
                asset: addr.lp_native_info_checked(),
                distribution: Decimal::from_ratio(1u128, 3u128),
                total_vp: u(10000)
              },
            ]
          },
          GaugeDistributionResponse {
            gauge: addr.gauge_3.to_string(),
            period: 76,
            total_gauge_vp: u(0),
            assets: vec![]
          }
        ]
      );
    });
}

#[test]
fn test_gauge_distributions_decay() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite
    .e_ve_create_lock_time_any(Some(SECONDS_PER_WEEK * 10), addr.ampluna(1000), "user1", |res| {
      res.assert_valid()
    })
    .def_staking_whitelist_recapture()
    .use_staking_2()
    .def_staking_whitelist_recapture()
    .use_staking_1()
    .def_gauge_1_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_2_vote(10000, 0, "user1", |res| res.assert_valid())
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .q_gauge_distributions(None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          GaugeDistributionResponse {
            gauge: addr.gauge_1.to_string(),
            period: 75,
            total_gauge_vp: u(2230),
            assets: vec![
              AssetDistribution {
                asset: addr.lp_cw20_info_checked(),
                // 0.5*1000+0.75*2000 = 2
                // we are automatically fixing rounding issues by adding it to the first
                distribution: Decimal::percent(50),
                total_vp: u(1115)
              },
              AssetDistribution {
                asset: addr.lp_native_info_checked(),
                // 0.5*1000+0.25*2000 = 1
                distribution: Decimal::percent(50),
                total_vp: u(1115)
              }
            ]
          },
          GaugeDistributionResponse {
            gauge: addr.gauge_2.to_string(),
            period: 75,
            total_gauge_vp: u(2230),
            assets: vec![AssetDistribution {
              asset: addr.lp_native_info_checked(),
              distribution: Decimal::one(),
              total_vp: u(2230)
            }]
          },
          GaugeDistributionResponse {
            gauge: addr.gauge_3.to_string(),
            period: 75,
            total_gauge_vp: u(0),
            assets: vec![]
          }
        ]
      );
    })
    .add_periods(5)
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .q_gauge_distributions(None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          GaugeDistributionResponse {
            gauge: addr.gauge_1.to_string(),
            period: 80,
            total_gauge_vp: u(1720),
            assets: vec![
              AssetDistribution {
                asset: addr.lp_cw20_info_checked(),
                // 0.5*1000+0.75*2000 = 2
                // we are automatically fixing rounding issues by adding it to the first
                distribution: Decimal::percent(50),
                total_vp: u(1720 / 2)
              },
              AssetDistribution {
                asset: addr.lp_native_info_checked(),
                // 0.5*1000+0.25*2000 = 1
                distribution: Decimal::percent(50),
                total_vp: u(1720 / 2)
              }
            ]
          },
          GaugeDistributionResponse {
            gauge: addr.gauge_2.to_string(),
            period: 80,
            // rounding issue
            total_gauge_vp: u(1715),
            assets: vec![AssetDistribution {
              asset: addr.lp_native_info_checked(),
              distribution: Decimal::one(),
              total_vp: u(1715)
            }]
          },
          GaugeDistributionResponse {
            gauge: addr.gauge_3.to_string(),
            period: 80,
            total_gauge_vp: u(0),
            assets: vec![]
          }
        ]
      );
    })
    .q_staking_reward_distribution(|res| {
      assert_eq!(
        res.unwrap(),
        vec![
          AssetDistribution {
            asset: addr.lp_cw20_info_checked(),
            distribution: Decimal::percent(50),
            total_vp: u(1720 / 2)
          },
          AssetDistribution {
            asset: addr.lp_native_info_checked(),
            distribution: Decimal::percent(50),
            total_vp: u(1720 / 2)
          }
        ]
      )
    })
    .use_staking_2()
    .q_staking_reward_distribution(|res| {
      assert_eq!(
        res.unwrap(),
        vec![AssetDistribution {
          asset: addr.lp_native_info_checked(),
          distribution: Decimal::one(),
          total_vp: u(1715)
        }]
      )
    });
}
