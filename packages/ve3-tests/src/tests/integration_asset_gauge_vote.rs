use crate::{
  common::{
    helpers::{cw20_info, native_info, u},
    suite::TestingSuite,
  },
  extensions::app_response_ext::Valid,
};
use ve3_shared::msgs_asset_gauge::*;

#[test]
fn test_vote_simple() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_time_any(None, addr.uluna(1200), "user1", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .use_staking_2()
    .def_staking_whitelist_recapture()
    .use_staking_1()
    .def_gauge_2_vote(10000, 0, "user1", |res| res.assert_valid())
    .def_gauge_1_vote(4000, 4000, "user1", |res| res.assert_valid())
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .q_gauge_user_shares("user1", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserSharesResponse {
          shares: vec![
            UserShare {
              gauge: "stable".to_string(),
              asset: addr.lp_native_info_checked(),
              period: 75,
              user_vp: u(4800),
              total_vp: u(4800)
            },
            UserShare {
              gauge: "stable".to_string(),
              asset: cw20_info(addr.lp_cw20.as_str()),
              period: 75,
              user_vp: u(4800),
              total_vp: u(4800)
            },
            UserShare {
              gauge: "project".to_string(),
              asset: addr.lp_native_info_checked(),
              period: 75,
              user_vp: u(12000),
              total_vp: u(12000)
            }
          ]
        },
      )
    })
    .def_gauge_1_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_2_vote(0, 0, "user1", |res| res.assert_valid())
    .add_one_period()
    .e_gauge_set_distribution("user2", |res| res.assert_valid())
    .q_gauge_user_shares(
      "user1",
      Some(ve3_shared::helpers::time::Times::Periods(vec![75, 76])),
      |res| {
        assert_eq!(
          res.unwrap(),
          UserSharesResponse {
            shares: vec![
              UserShare {
                gauge: "stable".to_string(),
                asset: addr.lp_native_info_checked(),
                period: 75,
                user_vp: u(4800),
                total_vp: u(4800)
              },
              UserShare {
                gauge: "stable".to_string(),
                asset: cw20_info(addr.lp_cw20.as_str()),
                period: 75,
                user_vp: u(4800),
                total_vp: u(4800)
              },
              UserShare {
                gauge: "project".to_string(),
                asset: addr.lp_native_info_checked(),
                period: 75,
                user_vp: u(12000),
                total_vp: u(12000)
              },
              UserShare {
                gauge: "stable".to_string(),
                asset: addr.lp_native_info_checked(),
                period: 76,
                user_vp: u(6000),
                total_vp: u(6000)
              },
              UserShare {
                gauge: "stable".to_string(),
                asset: cw20_info(addr.lp_cw20.as_str()),
                period: 76,
                user_vp: u(6000),
                total_vp: u(6000)
              },
              // removed
              // UserShare {
              //   gauge: "project".to_string(),
              //   asset: addr.lp_native_info_checked(),
              //   period: 76,
              //   user_vp: u(12000),
              //   total_vp: u(12000)
              // }
            ]
          },
        )
      },
    );
}

#[test]
fn test_vote_with_two_no_vote() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_time_any(None, addr.uluna(1200), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time_any(None, addr.uluna(1200), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .use_staking_2()
    .def_staking_whitelist_recapture()
    .use_staking_1()
    .def_gauge_2_vote(10000, 0, "user1", |res| res.assert_valid())
    .def_gauge_1_vote(4000, 4000, "user1", |res| res.assert_valid())
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .q_gauge_user_shares("user1", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserSharesResponse {
          shares: vec![
            UserShare {
              gauge: "stable".to_string(),
              asset: addr.lp_native_info_checked(),
              period: 75,
              user_vp: u(4800),
              total_vp: u(4800)
            },
            UserShare {
              gauge: "stable".to_string(),
              asset: cw20_info(addr.lp_cw20.as_str()),
              period: 75,
              user_vp: u(4800),
              total_vp: u(4800)
            },
            UserShare {
              gauge: "project".to_string(),
              asset: addr.lp_native_info_checked(),
              period: 75,
              user_vp: u(12000),
              total_vp: u(12000)
            }
          ]
        },
      )
    })
    .def_gauge_1_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_2_vote(0, 0, "user1", |res| res.assert_valid())
    .add_one_period()
    .e_gauge_set_distribution("user2", |res| res.assert_valid())
    .q_gauge_user_shares(
      "user1",
      Some(ve3_shared::helpers::time::Times::Periods(vec![75, 76])),
      |res| {
        assert_eq!(
          res.unwrap(),
          UserSharesResponse {
            shares: vec![
              UserShare {
                gauge: "stable".to_string(),
                asset: addr.lp_native_info_checked(),
                period: 75,
                user_vp: u(4800),
                total_vp: u(4800)
              },
              UserShare {
                gauge: "stable".to_string(),
                asset: cw20_info(addr.lp_cw20.as_str()),
                period: 75,
                user_vp: u(4800),
                total_vp: u(4800)
              },
              UserShare {
                gauge: "project".to_string(),
                asset: addr.lp_native_info_checked(),
                period: 75,
                user_vp: u(12000),
                total_vp: u(12000)
              },
              UserShare {
                gauge: "stable".to_string(),
                asset: addr.lp_native_info_checked(),
                period: 76,
                user_vp: u(6000),
                total_vp: u(6000)
              },
              UserShare {
                gauge: "stable".to_string(),
                asset: cw20_info(addr.lp_cw20.as_str()),
                period: 76,
                user_vp: u(6000),
                total_vp: u(6000)
              },
              // removed
              // UserShare {
              //   gauge: "project".to_string(),
              //   asset: addr.lp_native_info_checked(),
              //   period: 76,
              //   user_vp: u(12000),
              //   total_vp: u(12000)
              // }
            ]
          },
        )
      },
    );
}

#[test]
fn test_vote_with_two() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_time_any(None, addr.uluna(1200), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time_any(None, addr.uluna(1200), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .use_staking_2()
    .def_staking_whitelist_recapture()
    .use_staking_1()
    .def_gauge_2_vote(10000, 0, "user1", |res| res.assert_valid())
    .def_gauge_1_vote(4000, 4000, "user1", |res| res.assert_valid())
    .def_gauge_1_vote(5000, 5000, "user2", |res| res.assert_valid())
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .q_gauge_user_shares("user1", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserSharesResponse {
          shares: vec![
            UserShare {
              gauge: "stable".to_string(),
              asset: addr.lp_native_info_checked(),
              period: 75,
              user_vp: u(4800),
              total_vp: u(10800) // +6000 due to user2
            },
            UserShare {
              gauge: "stable".to_string(),
              asset: cw20_info(addr.lp_cw20.as_str()),
              period: 75,
              user_vp: u(4800),
              total_vp: u(10800) // +6000 due to user2
            },
            UserShare {
              gauge: "project".to_string(),
              asset: addr.lp_native_info_checked(),
              period: 75,
              user_vp: u(12000),
              total_vp: u(12000)
            }
          ]
        },
      )
    })
    .def_gauge_1_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_2_vote(0, 0, "user1", |res| res.assert_valid())
    .add_one_period()
    .e_gauge_set_distribution("user2", |res| res.assert_valid())
    .q_gauge_user_shares(
      "user1",
      Some(ve3_shared::helpers::time::Times::Periods(vec![75, 76])),
      |res| {
        assert_eq!(
          res.unwrap(),
          UserSharesResponse {
            shares: vec![
              UserShare {
                gauge: "stable".to_string(),
                asset: addr.lp_native_info_checked(),
                period: 75,
                user_vp: u(4800),
                total_vp: u(10800) // +6000 due to user2
              },
              UserShare {
                gauge: "stable".to_string(),
                asset: cw20_info(addr.lp_cw20.as_str()),
                period: 75,
                user_vp: u(4800),
                total_vp: u(10800) // +6000 due to user2
              },
              UserShare {
                gauge: "project".to_string(),
                asset: addr.lp_native_info_checked(),
                period: 75,
                user_vp: u(12000),
                total_vp: u(12000)
              },
              UserShare {
                gauge: "stable".to_string(),
                asset: addr.lp_native_info_checked(),
                period: 76,
                user_vp: u(6000),
                total_vp: u(12000) // +6000 due to user2
              },
              UserShare {
                gauge: "stable".to_string(),
                asset: cw20_info(addr.lp_cw20.as_str()),
                period: 76,
                user_vp: u(6000),
                total_vp: u(12000) // +6000 due to user2
              },
              // removed
              // UserShare {
              //   gauge: "project".to_string(),
              //   asset: addr.lp_native_info_checked(),
              //   period: 76,
              //   user_vp: u(12000),
              //   total_vp: u(12000)
              // }
            ]
          },
        )
      },
    );
}
