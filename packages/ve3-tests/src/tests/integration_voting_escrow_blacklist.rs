use crate::{
  common::{helpers::u, suite::TestingSuite},
  extensions::app_response_ext::{EventChecker, Valid},
};
use cosmwasm_std::attr;
use ve3_shared::{
  constants::{AT_VE_GUARDIAN, SECONDS_PER_WEEK},
  error::SharedError,
  helpers::time::Time,
  msgs_asset_gauge::UserInfoExtendedResponse,
};
use ve3_voting_escrow::error::ContractError;

#[test]
fn test_blacklist() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.ampluna(1000), "user1", |res| {
      res.unwrap();
    })
    .add_one_period()
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(103),
          fixed_amount: u(1200),
          slope: u(103),
          gauge_votes: vec![]
        }
      );
    })
    .e_ve_update_blacklist(
      Some(vec![addr.user1.to_string(), addr.user2.to_string()]),
      None,
      "creator",
      |res| {
        res.assert_attribute(attr("action", "ve/update_blacklist"));
        res.assert_attribute(attr("added_addresses", format!("{0},{1}", addr.user1, addr.user2)));
        res.assert_attribute(attr("action", "gauge/update_vote"));
      },
    )
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(0),
          fixed_amount: u(0),
          slope: u(0),
          gauge_votes: vec![]
        }
      );
    })
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.ampluna(1000), "user1", |res| {
      res.assert_error(ContractError::AddressBlacklisted(addr.user1.to_string()))
    })
    .add_one_period()
    .e_ve_withdraw("1", "user1", |res| {
      res.assert_attribute(attr("action", "ve/withdraw"));
      res.assert_attribute(attr("token_id", "1"));
      res.assert_attribute(attr("amount", "1000"));
      res.assert_attribute(attr("to", addr.user1.to_string()));
      res.assert_attribute(attr("action", "gauge/update_vote"));
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(0),
          fixed_amount: u(0),
          slope: u(0),
          gauge_votes: vec![]
        }
      );
    })
    .q_ve_blacklisted_voters(None, None, |res| {
      assert_eq!(res.unwrap(), vec![addr.user1.clone(), addr.user2.clone()])
    })
    .q_ve_blacklisted_voters(None, Some(1), |res| {
      assert_eq!(res.unwrap(), vec![addr.user1.clone()])
    })
    .q_ve_blacklisted_voters(Some(addr.user1.to_string()), None, |res| {
      assert_eq!(res.unwrap(), vec![addr.user2.clone()])
    });
}

#[test]
fn test_blacklist_remove() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.ampluna(1000), "user1", |res| {
      res.unwrap();
    })
    .add_one_period()
    .e_ve_update_blacklist(Some(vec![addr.user1.to_string()]), None, "creator", |res| {
      res.assert_valid()
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(0),
          fixed_amount: u(0),
          slope: u(0),
          gauge_votes: vec![]
        }
      );
    })
    .e_ve_update_blacklist(None, Some(vec![addr.user1.to_string()]), "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
        AT_VE_GUARDIAN.to_string(),
        addr.user1.to_string(),
      )))
    })
    .e_ve_update_blacklist(None, Some(vec![addr.user1.to_string()]), "AT_VE_GUARDIAN", |res| {
      res.assert_valid();
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(103),
          fixed_amount: u(1200),
          slope: u(103),
          gauge_votes: vec![]
        }
      );
    })
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user1", |res| {
      res.assert_valid()
    })
    .add_one_period()
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(86),
          fixed_amount: u(2200),
          slope: u(86),
          gauge_votes: vec![]
        }
      );
    })
    .q_ve_blacklisted_voters(None, None, |res| {
      assert!(res.unwrap().is_empty(), "no blacklisted voters")
    });
}
