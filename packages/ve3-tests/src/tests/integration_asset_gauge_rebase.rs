use crate::{
  common::{helpers::u, suite::TestingSuite},
  extensions::app_response_ext::{EventChecker, Valid},
};
use cosmwasm_std::{attr, Decimal};
use ve3_asset_gauge::error::ContractError;
use ve3_shared::{
  constants::WEEK,
  error::SharedError,
  helpers::time::Time,
  msgs_asset_gauge::*,
  msgs_voting_escrow::{End, LockInfoResponse},
};

#[test]
fn test_basic_rebase() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_get_ampluna("user1", 10000)
    .e_ve_create_lock_time(WEEK * 2, addr.ampluna(1000), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(2400), "user2", |res| res.assert_valid())
    .e_gauge_add_rebase_in_ampluna(3000, |res| {
      res.assert_attribute(attr("action", "gauge/add_rebase"));
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
    })
    .e_gauge_claim_rebase(Some("1"), "user3", |res| {
      res.assert_error(ContractError::SharedError(
        ve3_shared::error::SharedError::InsufficientBalance("no rebase amount".to_string()),
      ))
    })
    .e_gauge_claim_rebase(Some("1"), "user1", |res| {
      res.assert_error(ContractError::RebaseClaimingOnlyForPermanent)
    })
    .e_ve_lock_permanent("1", "user1", |res| {
      res.assert_attribute(attr("action", "ve/lock_permanent"));
      res.assert_attribute(attr("lock_end", "permanent"));
      res.assert_attribute(attr("fixed_power", "1200"));
      res.assert_attribute(attr("voting_power", "10800"));
    })
    .q_ve_lock_info("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 74,
          asset: addr.ampluna(1000),
          underlying_amount: u(1200),
          start: 74,
          end: End::Permanent,
          slope: u(0),
          fixed_amount: u(1200),
          voting_power: u(10800),
          coefficient: Decimal::percent(900)
        }
      );
    })
    .e_gauge_claim_rebase(Some("1"), "user1", |res| {
      res.assert_attribute(attr("action", "gauge/claim_rebase"));
      res.assert_attribute(attr("action", "ve/deposit_for"));
      res.assert_attribute(attr("action", "gauge/update_vote"));
      res.assert_attribute(attr("rebase_amount", "999"));
      res.assert_attribute(attr("fixed_power", "2398"));
      res.assert_attribute(attr("voting_power", "21582"));
    })
    .q_ve_lock_info("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 74,
          asset: addr.ampluna(1999),
          underlying_amount: u(2398),
          start: 74,
          end: End::Permanent,
          slope: u(0),
          fixed_amount: u(2398),
          voting_power: u(21582),
          coefficient: Decimal::percent(900)
        }
      );
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(21582),
          fixed_amount: u(2398),
          slope: u(0),
          gauge_votes: vec![]
        }
      )
    });
}

#[test]
fn test_rebase_new_lock() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_get_ampluna("user2", 10000)
    .e_ve_create_lock_time_any(None, addr.uluna(1200), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time(WEEK * 2, addr.ampluna(2000), "user2", |res| res.assert_valid())
    .e_gauge_add_rebase_in_ampluna(3000, |res| {
      res.assert_attribute(attr("action", "gauge/add_rebase"));
    })
    .e_gauge_claim_rebase(None, "user1", |res| {
      res.assert_attribute(attr("action", "gauge/claim_rebase"));
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("action", "gauge/update_vote"));
      res.assert_attribute(attr("rebase_amount", "999"));
      res.assert_attribute(attr("fixed_power", "1198"));
      res.assert_attribute(attr("voting_power", "10782"));
      res.assert_attribute(attr("token_id", "3"));
    })
    .q_ve_lock_info("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 74,
          asset: addr.uluna(1200),
          underlying_amount: u(1200),
          start: 74,
          end: End::Permanent,
          slope: u(0),
          fixed_amount: u(1200),
          voting_power: u(1200 * 9),
          coefficient: Decimal::percent(900)
        }
      );
    })
    .q_ve_lock_info("3", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 74,
          asset: addr.ampluna(999),
          underlying_amount: u(1198),
          start: 74,
          end: End::Permanent,
          slope: u(0),
          fixed_amount: u(1198),
          voting_power: u(1198 * 9),
          coefficient: Decimal::percent(900)
        }
      );
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u((1198 + 1200) * 9),
          fixed_amount: u(1198 + 1200),
          slope: u(0),
          gauge_votes: vec![]
        }
      )
    });
}

#[test]
fn test_rebase_new_lock_non_permanent() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_get_ampluna("user2", 10000)
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(1200), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time(WEEK * 2, addr.ampluna(2000), "user2", |res| res.assert_valid())
    .e_gauge_add_rebase_in_ampluna(3000, |res| {
      res.assert_attribute(attr("action", "gauge/add_rebase"));
    })
    .e_gauge_claim_rebase(None, "user1", |res| {
      res.assert_attribute(attr("action", "gauge/claim_rebase"));
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("action", "gauge/update_vote"));
      res.assert_attribute(attr("rebase_amount", "999"));
      res.assert_attribute(attr("fixed_power", "1198"));
      res.assert_attribute(attr("voting_power", "10782"));
      res.assert_attribute(attr("token_id", "3"));
    })
    .q_ve_lock_info("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 74,
          asset: addr.uluna(1200),
          underlying_amount: u(1200),
          start: 74,
          end: End::Period(76),
          slope: u(103),
          fixed_amount: u(1200),
          voting_power: u(103 * 2),
          ..res
        }
      );
    })
    .q_ve_lock_info("3", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 74,
          asset: addr.ampluna(999),
          underlying_amount: u(1198),
          start: 74,
          end: End::Permanent,
          slope: u(0),
          fixed_amount: u(1198),
          voting_power: u(10782),
          coefficient: Decimal::percent(900)
        }
      );
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(10988),
          fixed_amount: u(2398),
          slope: u(103),
          gauge_votes: vec![]
        }
      )
    });
}

#[test]
fn test_rebase_double_claim() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_get_ampluna("user2", 10000)
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(1200), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time(WEEK * 2, addr.ampluna(2000), "user2", |res| res.assert_valid())
    .e_gauge_add_rebase_in_ampluna(3000, |res| {
      res.assert_attribute(attr("action", "gauge/add_rebase"));
    })
    .e_gauge_claim_rebase(None, "user1", |res| {
      res.assert_attribute(attr("action", "gauge/claim_rebase"));
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("action", "gauge/update_vote"));
      res.assert_attribute(attr("rebase_amount", "999"));
      res.assert_attribute(attr("fixed_power", "1198"));
      res.assert_attribute(attr("voting_power", "10782"));
      res.assert_attribute(attr("token_id", "3"));
    })
    .e_gauge_claim_rebase(None, "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::InsufficientBalance(
        "no rebase amount".to_string(),
      )))
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

#[test]
fn test_rebase_claim_to_invalid_lock() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_get_ampluna("user2", 10000)
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(1200), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time(WEEK * 2, addr.ampluna(2000), "user2", |res| res.assert_valid())
    .e_gauge_add_rebase_in_ampluna(3000, |res| {
      res.assert_attribute(attr("action", "gauge/add_rebase"));
    })
    .e_gauge_claim_rebase(Some("2"), "user2", |res| {
      res.assert_error(ContractError::RebaseClaimingOnlyForPermanent)
    })
    .e_ve_lock_permanent("1", "user1", |res| res.assert_valid())
    .e_gauge_claim_rebase(Some("1"), "user1", |res| {
      res.assert_error(ContractError::RebaseWrongTargetLockAsset)
    })
    .e_gauge_claim_rebase(Some("21"), "user1", |res| res.assert_error(ContractError::LockNotFound))
    .e_gauge_claim_rebase(None, "user2", |res| {
      res.assert_attribute(attr("action", "gauge/claim_rebase"));
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("action", "mint"));
      res.assert_attribute(attr("owner", addr.user2.to_string()));
      res.assert_attribute(attr("action", "gauge/update_vote"));
      res.assert_attribute(attr("rebase_amount", "1999"));
      res.assert_attribute(attr("fixed_power", "2398"));
      res.assert_attribute(attr("voting_power", "21582"));
      res.assert_attribute(attr("token_id", "3"));
    })
    .q_gauge_user_pending_rebase("user2", |res| {
      assert_eq!(
        res.unwrap(),
        UserPendingRebaseResponse {
          rebase: u(0)
        }
      );
    });
}
