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
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(1200), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time(WEEK * 2, addr.ampluna(2000), "user2", |res| res.assert_valid())
    .e_gauge_add_rebase("creator", addr.uluna(3000), |res| {
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
          asset: addr.uluna(1200),
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
      res.assert_attribute(attr("fixed_power", "2199"));
    })
    .q_ve_lock_info("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 74,
          asset: addr.uluna(2199),
          underlying_amount: u(2199),
          start: 74,
          end: End::Permanent,
          slope: u(0),
          fixed_amount: u(2199),
          voting_power: u(19791),
          coefficient: Decimal::percent(900)
        }
      );
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(19791),
          fixed_amount: u(2199),
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
    .e_ve_create_lock_time_any(None, addr.uluna(1200), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time(WEEK * 2, addr.ampluna(2000), "user2", |res| res.assert_valid())
    .e_gauge_add_rebase("creator", addr.uluna(3000), |res| {
      res.assert_attribute(attr("action", "gauge/add_rebase"));
    })
    .e_gauge_claim_rebase(None, "user1", |res| {
      res.assert_attribute(attr("action", "gauge/claim_rebase"));
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("action", "gauge/update_vote"));
      res.assert_attribute(attr("rebase_amount", "999"));
      res.assert_attribute(attr("fixed_power", "999"));
      res.assert_attribute(attr("voting_power", "8991"));
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
          voting_power: u(10800),
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
          asset: addr.uluna(999),
          underlying_amount: u(999),
          start: 74,
          end: End::Permanent,
          slope: u(0),
          fixed_amount: u(999),
          voting_power: u(8991),
          coefficient: Decimal::percent(900)
        }
      );
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(19791),
          fixed_amount: u(2199),
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
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(1200), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time(WEEK * 2, addr.ampluna(2000), "user2", |res| res.assert_valid())
    .e_gauge_add_rebase("creator", addr.uluna(3000), |res| {
      res.assert_attribute(attr("action", "gauge/add_rebase"));
    })
    .e_gauge_claim_rebase(None, "user1", |res| {
      res.assert_attribute(attr("action", "gauge/claim_rebase"));
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("action", "gauge/update_vote"));
      res.assert_attribute(attr("rebase_amount", "999"));
      res.assert_attribute(attr("fixed_power", "999"));
      res.assert_attribute(attr("voting_power", "8991"));
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
          voting_power: u(206),
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
          asset: addr.uluna(999),
          underlying_amount: u(999),
          start: 74,
          end: End::Permanent,
          slope: u(0),
          fixed_amount: u(999),
          voting_power: u(8991),
          coefficient: Decimal::percent(900)
        }
      );
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(9197),
          fixed_amount: u(2199),
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
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(1200), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time(WEEK * 2, addr.ampluna(2000), "user2", |res| res.assert_valid())
    .e_gauge_add_rebase("creator", addr.uluna(3000), |res| {
      res.assert_attribute(attr("action", "gauge/add_rebase"));
    })
    .e_gauge_claim_rebase(None, "user1", |res| {
      res.assert_attribute(attr("action", "gauge/claim_rebase"));
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("action", "gauge/update_vote"));
      res.assert_attribute(attr("rebase_amount", "999"));
      res.assert_attribute(attr("fixed_power", "999"));
      res.assert_attribute(attr("voting_power", "8991"));
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
    .e_ve_create_lock_time(WEEK * 2, addr.uluna(1200), "user1", |res| res.assert_valid())
    .e_ve_create_lock_time(WEEK * 2, addr.ampluna(2000), "user2", |res| res.assert_valid())
    .e_gauge_add_rebase("creator", addr.uluna(3000), |res| {
      res.assert_attribute(attr("action", "gauge/add_rebase"));
    })
    .e_gauge_claim_rebase(Some("2"), "user2", |res| {
      res.assert_error(ContractError::RebaseClaimingOnlyForPermanent)
    })
    .e_ve_lock_permanent("2", "user2", |res| res.assert_valid())
    .e_gauge_claim_rebase(Some("2"), "user2", |res| {
      res.assert_error(ContractError::RebaseWrongTargetLockAsset)
    })
    .e_gauge_claim_rebase(None, "user2", |res| {
      res.assert_attribute(attr("action", "gauge/claim_rebase"));
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("action", "mint"));
      res.assert_attribute(attr("owner", addr.user2.to_string()));
      res.assert_attribute(attr("action", "gauge/update_vote"));
      res.assert_attribute(attr("rebase_amount", "1999"));
      res.assert_attribute(attr("fixed_power", "1999"));
      res.assert_attribute(attr("voting_power", "17991"));
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
