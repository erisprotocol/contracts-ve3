use crate::{
  common::{
    helpers::u,
    suite::{InitOptions, TestingSuite},
  },
  extensions::app_response_ext::{EventChecker, Valid},
};
use cosmwasm_std::{attr, Decimal, Uint128};
use ve3_connector_emission::error::ContractError;
use ve3_shared::{
  constants::{at_asset_staking, SECONDS_PER_YEAR},
  error::SharedError,
  extensions::asset_info_ext::AssetInfoExt,
  msgs_asset_gauge::UserPendingRebaseResponse,
  msgs_connector_emission::{Config, RebaseConfg},
};

#[test]
fn test_emission_connector_rebase_wrong() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .use_connector_emissions()
    .use_staking_3()
    .e_ve_create_lock_time_any(None, addr.uluna(1000), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time_any(None, addr.uluna(2000), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .def_gauge_3_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_3_vote(7500, 2500, "user2", |res| res.assert_valid())
    .e_emission_update_config(None, None, None, None, Some(true), None, "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::Unauthorized {}))
    })
    .e_emission_update_config(None, None, None, None, Some(true), None, "creator", |res| {
      res.assert_valid();
    })
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    // claiming
    .e_emission_claim_rewards("user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
        at_asset_staking(&addr.gauge_3),
        addr.user1.to_string(),
      )))
    })
    .e_staking_update_rewards("user1", |res| {
      res.assert_error(ve3_asset_gauge::error::ContractError::SharedError(
        SharedError::WrongDeposit("wrong deposit uluna".to_string()),
      ))
    });
}

#[test]
fn test_emission_connector_rebase() {
  let mut suite = TestingSuite::def();
  suite.init_options(InitOptions {
    rebase_asset: Some(suite.addresses.uluna_info()),
  });

  let addr = suite.addresses.clone();

  suite
    .use_connector_emissions()
    .use_staking_3()
    .e_ve_create_lock_time_any(None, addr.uluna(1000), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time_any(None, addr.uluna(2000), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .def_gauge_3_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_3_vote(7500, 2500, "user2", |res| res.assert_valid())
    .e_emission_update_config(None, None, None, None, Some(true), None, "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::Unauthorized {}))
    })
    .e_emission_update_config(None, None, None, None, Some(true), None, "creator", |res| {
      res.assert_valid();
    })
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    // claiming
    .e_emission_claim_rewards("user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
        at_asset_staking(&addr.gauge_3),
        addr.user1.to_string(),
      )))
    })
    .e_staking_update_rewards("user1", |res| {
      res.assert_attribute(attr("action", "asset/update_rewards"));
      res.assert_attribute(attr("action", "ce/claim_rewards"));
      res.assert_attribute(attr("emission_amount", "100"));
      res.assert_attribute(attr("rebase_amount", "49"));
      res.assert_attribute(attr("team_amount", "10"));
      res.assert_attribute(attr("action", "gauge/add_rebase"));
      res.assert_attribute(attr("rebase", "native:uluna:49"));
      res.assert_attribute(attr("action", "asset/update_rewards_callback"));
    })
    .e_staking_update_rewards("user1", |res| {
      res.assert_attribute(attr("action", "asset/update_rewards"));
      res.assert_attribute(attr("action", "ce/claim_rewards_noop"));
      res.assert_attribute(attr("action", "asset/update_rewards_callback"));
    })
    .q_gauge_user_pending_rebase("user1", |res| {
      assert_eq!(
        res.unwrap(),
        UserPendingRebaseResponse {
          rebase: u(16)
        }
      );
    })
    .q_gauge_user_pending_rebase("user2", |res| {
      assert_eq!(
        res.unwrap(),
        UserPendingRebaseResponse {
          rebase: u(32)
        }
      );
    })
    .e_gauge_claim_rebase(None, "user2", |res| {
      res.assert_attribute(attr("action", "gauge/claim_rebase"));
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("action", "gauge/update_vote"));
      res.assert_attribute(attr("rebase_amount", "32"));
      res.assert_attribute(attr("fixed_power", "32"));
      res.assert_attribute(attr("voting_power", "288"));
    })
    .q_gauge_user_pending_rebase("user2", |res| {
      assert_eq!(
        res.unwrap(),
        UserPendingRebaseResponse {
          rebase: u(0)
        }
      );
    })
    .q_emission_config(|res| {
      assert_eq!(
        res.unwrap(),
        Config {
          global_config_addr: addr.ve3_global_config.clone(),
          gauge: addr.gauge_3.clone(),
          emissions_per_week: u(100),
          team_share: Decimal::percent(10),
          enabled: true,
          rebase_config: ve3_shared::msgs_connector_emission::RebaseConfg::Dynamic {},
          mint_config: ve3_shared::msgs_connector_emission::MintConfig::MintDirect,
          last_claim_s: 1712847600,
          emission_token: addr.uluna_info_checked()
        }
      );
    })
    .add_periods(100)
    .e_ve_create_lock_time_any(
      None,
      addr.uluna_info_checked().with_balance(Uint128::new(990000_000000u128)),
      "user2",
      |res| res.assert_valid(),
    )
    .e_staking_update_rewards("user1", |res| {
      res.assert_attribute(attr("action", "asset/update_rewards"));
      res.assert_attribute(attr("action", "ce/claim_rewards"));
      res.assert_attribute(attr("emission_amount", "10000"));
      res.assert_attribute(attr("rebase_amount", "4999"));
      res.assert_attribute(attr("team_amount", "1000"));
      res.assert_attribute(attr("action", "gauge/add_rebase"));
      res.assert_attribute(attr("rebase", "native:uluna:4999"));
      res.assert_attribute(attr("action", "asset/update_rewards_callback"));
      res.assert_attribute(attr("rewards", "native:uluna:10000"));
    });
}

#[test]
fn test_emission_connector_rebase_fixed() {
  let mut suite = TestingSuite::def();
  suite.init_options(InitOptions {
    rebase_asset: Some(suite.addresses.uluna_info()),
  });

  let addr = suite.addresses.clone();

  suite
    .use_connector_emissions()
    .use_staking_3()
    .e_ve_create_lock_time_any(None, addr.uluna(1000), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time_any(None, addr.uluna(2000), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .def_gauge_3_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_3_vote(7500, 2500, "user2", |res| res.assert_valid())
    .e_emission_update_config(
      None,
      None,
      Some(RebaseConfg::Fixed(Decimal::percent(20))),
      None,
      Some(true),
      None,
      "creator",
      |res| {
        res.assert_valid();
      },
    )
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .e_staking_update_rewards("user1", |res| {
      res.assert_attribute(attr("action", "asset/update_rewards"));
      res.assert_attribute(attr("action", "ce/claim_rewards"));
      res.assert_attribute(attr("emission_amount", "100"));
      res.assert_attribute(attr("rebase_amount", "20"));
      res.assert_attribute(attr("team_amount", "10"));
      res.assert_attribute(attr("action", "gauge/add_rebase"));
      res.assert_attribute(attr("rebase", "native:uluna:20"));
      res.assert_attribute(attr("action", "asset/update_rewards_callback"));
      res.assert_attribute(attr("rewards", "native:uluna:100"));
    });
}

#[test]
fn test_emission_connector_rebase_apy() {
  let mut suite = TestingSuite::def();
  suite.init_options(InitOptions {
    rebase_asset: Some(suite.addresses.uluna_info()),
  });

  let addr = suite.addresses.clone();

  suite
    .use_connector_emissions()
    .use_staking_3()
    .e_ve_create_lock_time_any(None, addr.uluna(8000), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time_any(None, addr.uluna(2000), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .def_gauge_3_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_3_vote(7500, 2500, "user2", |res| res.assert_valid())
    .e_emission_update_config(
      None,
      None,
      Some(RebaseConfg::TargetYearlyApy(Decimal::percent(8))),
      None,
      Some(true),
      None,
      "creator",
      |res| {
        res.assert_valid();
      },
    )
    .add_seconds(SECONDS_PER_YEAR)
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .e_staking_update_rewards("user1", |res| {
      // 10000 staked
      // 8 % target ->
      res.assert_attribute(attr("action", "asset/update_rewards"));
      res.assert_attribute(attr("action", "ce/claim_rewards"));
      // 100 per week
      res.assert_attribute(attr("emission_amount", "5214"));
      res.assert_attribute(attr("rebase_amount", "800"));
      res.assert_attribute(attr("team_amount", "521"));
      res.assert_attribute(attr("action", "gauge/add_rebase"));
      res.assert_attribute(attr("rebase", "native:uluna:800"));
      res.assert_attribute(attr("action", "asset/update_rewards_callback"));
      res.assert_attribute(attr("rewards", "native:uluna:5214"));
    });
}
