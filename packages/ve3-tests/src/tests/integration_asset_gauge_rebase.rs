use crate::{
  common::{helpers::u, suite::TestingSuite},
  extensions::app_response_ext::{EventChecker, Valid},
};
use cosmwasm_std::attr;
use ve3_shared::{ constants::WEEK, msgs_asset_gauge::UserPendingRebaseResponse};

#[test]
fn test_basic_rebase() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(1200), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time(WEEK * 2, addr.ampluna(2000), "user2", |res| res.assert_valid())
    .e_gauge_add_rebase("creator", addr.uluna(3000), |res| {
      res.assert_attribute(attr("action", "gauge/add_rebase")).unwrap();
    })
    .q_gauge_user_pending_rebase("user1", |res| {
      assert_eq!(
        res.unwrap(),
        UserPendingRebaseResponse {
          rebase: u(999)
        }
      );
    })
    .q_gauge_user_pending_rebase("user2", |res| {
      assert_eq!(
        res.unwrap(),
        UserPendingRebaseResponse {
          rebase: u(1999)
        }
      );
    });
}
