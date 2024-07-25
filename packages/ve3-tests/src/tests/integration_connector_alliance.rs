use crate::{
  common::{helpers::u, suite::TestingSuite},
  extensions::{
    app_response_ext::{EventChecker, Valid},
    helpers::assert_close,
  },
};
use cosmwasm_std::{attr, Decimal};
use ve3_connector_alliance::error::ContractError;
use ve3_shared::{
  constants::at_asset_staking, error::SharedError, msgs_asset_gauge::UserPendingRebaseResponse,
  msgs_connector_alliance::*,
};

#[test]
fn test_alliance_connector_rebase() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();
  let ampluna = addr.eris_hub_cw20_ampluna.to_string();

  suite
    .use_connector_alliance_eris()
    .use_staking_2()
    .e_ve_create_lock_time_any(None, addr.uluna(1000), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time_any(None, addr.uluna(2000), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .def_gauge_2_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_2_vote(7500, 2500, "user2", |res| res.assert_valid())
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    // claiming
    .e_alliance_claim_rewards("user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
        at_asset_staking(&addr.gauge_2),
        addr.user1.to_string(),
      )))
    })
    .e_staking_update_rewards("user1", |res| {
      res.assert_attribute(attr("action", "asset/update_rewards"));
      res.assert_attribute(attr("action", "ca/claim_rewards"));
      res.assert_attribute(attr("action", "ca/claim_rewards_callback"));
      res.assert_attribute(attr("claimed", "native:uluna:0"));
      res.assert_attribute(attr("action", "ca/bond_rewards_callback"));
      res.assert_attribute(attr("share", "0"));
      res.assert_attribute(attr("action", "asset/update_rewards_callback"));
    })
    .def_send("creator", addr.ve3_connector_alliance_eris.clone(), addr.uluna(10000))
    .e_staking_update_rewards("user1", |res| {
      res.assert_attribute(attr("action", "asset/update_rewards"));
      res.assert_attribute(attr("action", "ca/claim_rewards"));
      res.assert_attribute(attr("action", "ca/claim_rewards_callback"));
      res.assert_attribute(attr("claimed", "native:uluna:10000"));
      res.assert_attribute(attr("action", "erishub/bond"));
      res.assert_attribute(attr("action", "ca/bond_rewards_callback"));
      res.assert_attribute(attr("share", "8333"));
      res.assert_attribute(attr("amount", "8333"));
      res.assert_attribute(attr("action", "asset/update_rewards_callback"));
    })
    .q_alliance_state(|res| {
      let res = res.unwrap();
      assert_close(res.last_exchange_rate, Decimal::percent(120), Decimal::permille(1));
      assert_eq!(
        res.clone(),
        StateResponse {
          taken: u(0),
          harvested: u(0),
          ..res
        }
      )
    })
    .add_periods(52)
    .def_harvest()
    .q_alliance_state(|res| {
      let res = res.unwrap();
      assert_close(res.last_exchange_rate, Decimal::percent(120), Decimal::permille(1));
      assert_eq!(
        res.clone(),
        StateResponse {
          taken: u(0),
          harvested: u(0),
          ..res
        }
      )
    })
    .e_alliance_distribute_rebase(Some(true), "user1", |res| {
      res.assert_attribute(attr("action", "ca/distribute_rebase"));
      res.assert_attribute(attr("action", "gauge/add_rebase"));
      res.assert_attribute(attr("rebase", format!("cw20:{ampluna}:666")));
    })
    .q_alliance_state(|res| {
      let res = res.unwrap();
      assert_close(res.last_exchange_rate, Decimal::permille(1304), Decimal::permille(1));
      assert_eq!(
        res.clone(),
        StateResponse {
          taken: u(666),
          harvested: u(666),
          ..res
        }
      )
    })
    .add_periods(5)
    .def_harvest()
    .def_send("creator", addr.ve3_connector_alliance_eris.clone(), addr.uluna(10000))
    .e_staking_update_rewards("user1", |res| {
      res.assert_attribute(attr("action", "asset/update_rewards"));
      res.assert_attribute(attr("action", "ca/claim_rewards"));
      res.assert_attribute(attr("action", "ca/claim_rewards_callback"));
      res.assert_attribute(attr("claimed", "native:uluna:10000"));
      res.assert_attribute(attr("action", "erishub/bond"));
      res.assert_attribute(attr("action", "ca/bond_rewards_callback"));
      res.assert_attribute(attr("share", "8331"));
      res.assert_attribute(attr("amount", "7604"));
      res.assert_attribute(attr("action", "asset/update_rewards_callback"));
    })
    .q_alliance_state(|res| {
      let res = res.unwrap();
      assert_close(res.last_exchange_rate, Decimal::permille(1314), Decimal::permille(1));
      assert_eq!(
        res.clone(),
        StateResponse {
          taken: u(728),
          harvested: u(666),
          ..res
        }
      )
    })
    .q_gauge_user_pending_rebase("user1", |res| {
      assert_eq!(
        res.unwrap(),
        UserPendingRebaseResponse {
          rebase: u(222)
        }
      );
    })
    .q_gauge_user_pending_rebase("user2", |res| {
      assert_eq!(
        res.unwrap(),
        UserPendingRebaseResponse {
          rebase: u(444)
        }
      );
    })
    .e_gauge_claim_rebase(None, "user2", |res| {
      res.assert_attribute(attr("action", "gauge/claim_rebase"));
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("action", "gauge/update_vote"));
      res.assert_attribute(attr("rebase_amount", "444"));
      res.assert_attribute(attr("fixed_power", "583"));
      res.assert_attribute(attr("voting_power", "5247"));
    })
    .q_gauge_user_pending_rebase("user2", |res| {
      assert_eq!(
        res.unwrap(),
        UserPendingRebaseResponse {
          rebase: u(0)
        }
      );
    })
    .e_alliance_distribute_rebase(Some(true), "user1", |res| {
      res.assert_attribute(attr("action", "ca/distribute_rebase"));
      res.assert_attribute(attr("action", "gauge/add_rebase"));
      res.assert_attribute(attr("rebase", format!("cw20:{ampluna}:62")));
    })
    .add_one_period()
    .def_harvest()
    .e_alliance_distribute_rebase(Some(true), "user1", |res| {
      res.assert_attribute(attr("action", "ca/distribute_rebase"));
      res.assert_attribute(attr("action", "gauge/add_rebase"));
      res.assert_attribute(attr("rebase", format!("cw20:{ampluna}:24")));
    })
    .add_one_period()
    .def_harvest()
    .e_alliance_distribute_rebase(Some(true), "user1", |res| {
      res.assert_attribute(attr("action", "ca/distribute_rebase"));
      res.assert_attribute(attr("action", "gauge/add_rebase"));
      res.assert_attribute(attr("rebase", format!("cw20:{ampluna}:24")));
    })
    .add_one_period()
    .def_harvest()
    .e_alliance_distribute_rebase(None, "user1", |res| {
      res.assert_error(ContractError::NothingToTake)
    });
}

#[test]
fn test_alliance_connector_staking_rewards() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();
  suite
    .def_setup_staking()
    // stake some
    .e_staking_stake(None, addr.lp_cw20(1000), "user1", |res| {
      res.assert_attribute(attr("action", "asset/stake"));
      res.assert_attribute(attr("share", "1000"));
    })
    .e_staking_stake(None, addr.lp_native(1000), "user2", |res| {
      res.assert_attribute(attr("action", "asset/stake"));
      res.assert_attribute(attr("share", "1000"));
    })
    // update rewards
    .e_staking_claim_reward(addr.lp_cw20_info_checked(), "user2", |res| {
      res.assert_attribute(attr("action", "asset/claim_rewards"));
      res.assert_attribute(attr("assets", addr.lp_cw20_info_checked().to_string()));
      res.assert_attribute(attr("reward_amount", "0"));
    })
    .e_staking_claim_rewards(None, "user2", |res| {
      res.assert_attribute(attr("action", "asset/claim_rewards"));
      res.assert_attribute(attr(
        "assets",
        format!("{0},{1}", addr.lp_cw20_info_checked(), addr.lp_native_info_checked()),
      ));
      res.assert_attribute(attr("reward_amount", "0"));
    })
    .def_add_staking_rewards(120000)
    .e_staking_claim_rewards(None, "user1", |res| {
      res.assert_attribute(attr("action", "asset/claim_rewards"));
      res.assert_attribute(attr("assets", format!("{0}", addr.lp_cw20_info_checked())));
      res.assert_attribute(attr("reward_amount", "33332"));
    })
    .e_staking_claim_rewards(None, "user2", |res| {
      res.assert_attribute(attr("action", "asset/claim_rewards"));
      res.assert_attribute(attr(
        "assets",
        format!("{0},{1}", addr.lp_cw20_info_checked(), addr.lp_native_info_checked()),
      ));
      res.assert_attribute(attr("reward_amount", "66666"));
    });
}
