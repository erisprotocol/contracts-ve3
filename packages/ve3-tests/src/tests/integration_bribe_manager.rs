use crate::{common::suite::TestingSuite, extensions::app_response_ext::Valid};
use ve3_shared::constants::WEEK;

#[test]
fn test_lock_add_bribes() {
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
    .e_gauge_set_distribution("user1", |res| res.assert_valid());
}
