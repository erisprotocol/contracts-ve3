use crate::{
  common::{helpers::u, suite::TestingSuite},
  extensions::app_response_ext::{EventChecker, Valid},
};
use cosmwasm_std::{attr, Decimal};
use ve3_shared::msgs_asset_staking::*;

#[test]
fn test_staking_claim() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    // rewards: 3 for native : 1 for cw20
    .def_setup_staking()
    .e_staking_claim_reward(addr.lp_cw20_info_checked(), "user1", |res| res.assert_valid())
    .e_staking_stake(None, addr.lp_cw20(1000), "user2", |res| res.assert_valid())
    .def_add_staking_rewards(120000)
    .add_one_period()
    .e_staking_stake(None, addr.lp_cw20(1000), "user1", |res| res.assert_valid())
    .q_staking_config(|res| {
      assert_eq!(
        res.unwrap(),
        Config {
          reward_info: addr.zasset_info_checked(),
          global_config_addr: addr.ve3_global_config.clone(),
          default_yearly_take_rate: Decimal::percent(10),
          gauge: addr.gauge_2.to_string()
        }
      )
    })
    .q_staking_all_pending_rewards(
      AllPendingRewardsQuery {
        address: addr.user2.to_string(),
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![PendingRewardsRes {
            staked_asset_share: addr.lp_cw20(1000),
            reward_asset: addr.zasset(33332),
          }]
        )
      },
    )
    .q_staking_all_pending_rewards(
      AllPendingRewardsQuery {
        address: addr.user1.to_string(),
      },
      |res| assert_eq!(res.unwrap(), vec![]),
    );
}

#[test]
fn test_staking_claim_native() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    // rewards: 2 for native : 1 for cw20
    .def_setup_staking()
    .e_staking_claim_reward(addr.lp_native_info_checked(), "user1", |res| res.assert_valid())
    .e_staking_stake(None, addr.lp_native(1000), "user2", |res| res.assert_valid())
    .def_add_staking_rewards(120000)
    .add_one_period()
    .e_staking_stake(None, addr.lp_native(1000), "user1", |res| res.assert_valid())
    .q_staking_all_pending_rewards(
      AllPendingRewardsQuery {
        address: addr.user2.to_string(),
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![PendingRewardsRes {
            staked_asset_share: addr.lp_native(1000),
            reward_asset: addr.zasset(66666),
          }]
        )
      },
    )
    .q_staking_all_pending_rewards_details(
      AllPendingRewardsQuery {
        address: addr.user1.to_string(),
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![PendingRewardsDetailRes {
            // yearly take = 20% (higher than case 1)
            share: u(1003),
            // rounding difference
            staked_asset: addr.lp_native(999),
            reward_asset: addr.zasset(0),
          }]
        );
      },
    );
}

#[test]
fn test_staking_claim_both() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    // rewards: 2 for native : 1 for cw20
    .def_setup_staking()
    .e_staking_claim_reward(addr.lp_native_info_checked(), "user1", |res| res.assert_valid())
    .e_staking_claim_reward(addr.lp_cw20_info_checked(), "user1", |res| res.assert_valid())
    .e_staking_stake(None, addr.lp_native(1000), "user2", |res| res.assert_valid())
    .e_staking_stake(Some("user1"), addr.lp_cw20(1000), "user2", |res| res.assert_valid())
    .e_staking_stake(None, addr.lp_cw20(1000), "user2", |res| res.assert_valid())
    .def_add_staking_rewards(120000)
    .add_one_period()
    .e_staking_stake(None, addr.lp_cw20(1000), "user1", |res| res.assert_valid())
    .q_staking_all_pending_rewards(
      AllPendingRewardsQuery {
        address: addr.user2.to_string(),
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![
            PendingRewardsRes {
              staked_asset_share: addr.lp_cw20(1000),
              reward_asset: addr.zasset(16666),
            },
            PendingRewardsRes {
              staked_asset_share: addr.lp_native(1000),
              reward_asset: addr.zasset(66666),
            },
          ]
        )
      },
    )
    .q_staking_all_pending_rewards(
      AllPendingRewardsQuery {
        address: addr.user1.to_string(),
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![PendingRewardsRes {
            staked_asset_share: addr.lp_cw20(2001),
            reward_asset: addr.zasset(16666),
          }]
        )
      },
    )
    .e_staking_claim_rewards(None, "user1", |res| {
      res.assert_attribute(attr("action", "asset/claim_rewards"));
      res.assert_attribute_ty(
        "transfer",
        attr(
          "amount",
          "16666factory/terra1gurgpv8savnfw66lckwzn4zk7fp394lpe667dhu7aw48u40lj6jsln7pjn/zluna",
        ),
      );
      res.assert_attribute_ty("transfer", attr("recipient", addr.user1.to_string()));
    })
    .e_staking_claim_rewards(None, "user2", |res| {
      res.assert_attribute(attr("action", "asset/claim_rewards"));
      res.assert_attribute_ty(
        "transfer",
        attr(
          "amount",
          "83332factory/terra1gurgpv8savnfw66lckwzn4zk7fp394lpe667dhu7aw48u40lj6jsln7pjn/zluna",
        ),
      );
      res.assert_attribute_ty("transfer", attr("recipient", addr.user2.to_string()));
    })
    .e_alliance_withdraw("user2", 83332, |res| {
      res.assert_attribute(attr("action", "ca/withdraw"));
      res.assert_attribute(attr("amount", "83332"));
      res.assert_attribute(attr("share", "83332"));
      res.assert_attribute(attr("action", "transfer"));
      res.assert_attribute(attr("to", addr.user2.to_string()));
    })
    .q_staking_all_pending_rewards(
      AllPendingRewardsQuery {
        address: addr.user2.to_string(),
      },
      |res| assert_eq!(res.unwrap(), vec![]),
    )
    .q_staking_all_pending_rewards(
      AllPendingRewardsQuery {
        address: addr.user1.to_string(),
      },
      |res| assert_eq!(res.unwrap(), vec![]),
    )
    .q_staking_all_pending_rewards_details(
      AllPendingRewardsQuery {
        address: addr.user2.to_string(),
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![
            PendingRewardsDetailRes {
              share: u(1000),
              staked_asset: addr.lp_cw20(998),
              reward_asset: addr.zasset(0),
            },
            PendingRewardsDetailRes {
              share: u(1000),
              staked_asset: addr.lp_native(997),
              reward_asset: addr.zasset(0),
            },
          ]
        )
      },
    )
    .q_staking_all_pending_rewards_details(
      AllPendingRewardsQuery {
        address: addr.user1.to_string(),
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![PendingRewardsDetailRes {
            share: u(2001),
            staked_asset: addr.lp_cw20(1998),
            reward_asset: addr.zasset(0),
          },]
        )
      },
    )
    .add_periods(52)
    .def_harvest()
    .e_alliance_withdraw("user1", 16666, |res| {
      res.assert_attribute(attr("action", "ca/withdraw"));
      res.assert_attribute(attr("amount", "15311"));
      res.assert_attribute(attr("share", "16666"));
      res.assert_attribute(attr("action", "transfer"));
      res.assert_attribute(attr("to", addr.user1.to_string()));
    });
}
