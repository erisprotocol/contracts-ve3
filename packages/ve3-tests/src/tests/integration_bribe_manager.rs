use crate::{
  common::{
    helpers::{native, u, uluna},
    suite::TestingSuite,
  },
  extensions::app_response_ext::Valid,
};
use cosmwasm_std::{Addr, Decimal};
use cw_asset::AssetInfo;
use ve3_asset_staking::error::ContractError;
use ve3_shared::{
  constants::{AT_ASSET_WHITELIST_CONTROLLER, WEEK},
  error::SharedError,
  msgs_bribe_manager::*,
};

#[test]
fn test_lock_add_bribes() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_time(WEEK * 2, uluna(1000), "user1", |res| res.assert_valid())
    .add_one_period()
    .e_ve_create_lock_time(WEEK * 2, uluna(2000), "user2", |res| res.assert_valid())
    .def_staking_whitelist_recapture()
    .def_gauge_vote(5000, 5000, "user1", |res| res.assert_valid())
    .def_gauge_vote(7500, 2500, "user2", |res| res.assert_valid())
    // .e_bribe_add_bribe_native(bribe, gauge, for_info, distribution, sender, result)
    .add_one_period()
    .e_gauge_set_distribution("user1", |res| res.assert_valid())
    .e_gauge_set_distribution("user1", |res| res.assert_valid());
}
