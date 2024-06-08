use crate::{
  common::{
    helpers::{native, u},
    suite::TestingSuite,
  },
  extensions::app_response_ext::EventChecker,
};
use cosmwasm_std::{attr, Decimal};
use cw721::{AllNftInfoResponse, NftInfoResponse, OwnerOfResponse, TokensResponse};
use ve3_shared::{
  constants::{MAX_LOCK_PERIODS, WEEK},
  extensions::decimal_ext::DecimalExt,
  helpers::{slope::adjust_vp_and_slope, time::Time},
  msgs_asset_gauge::UserInfoExtendedResponse,
  msgs_voting_escrow::*,
};

#[test]
fn test_vote() {
  let mut suite = TestingSuite::def();

  let user1 = suite.address("user1").to_string();

  suite
    .init()
    .ve_create_lock_execute(WEEK * 2, native("uluna", 1000u128), "user1", |res| {
      res.unwrap();
    })
    .ve_create_lock_execute(WEEK * 2, native("uluna", 1000u128), "user2", |res| {
      res.unwrap();
    })
    .query_ve_all_tokens(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        TokensResponse {
          tokens: vec!["1".to_string(), "2".to_string()]
        }
      )
    })
    .query_ve_all_nft_info("1".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        AllNftInfoResponse::<Extension> {
          access: OwnerOfResponse {
            owner: user1.clone(),
            approvals: vec![]
          },
          info: NftInfoResponse {
            token_uri: None,
            extension: Extension {
              image: None,
              description: None,
              name: None,
              attributes: Some(vec![
                Trait {
                  display_type: None,
                  trait_type: "asset".to_string(),
                  value: "native:uluna:1000".to_string()
                },
                Trait {
                  display_type: None,
                  trait_type: "start".to_string(),
                  value: "74".to_string()
                },
                Trait {
                  display_type: None,
                  trait_type: "end".to_string(),
                  value: "76".to_string()
                }
              ])
            }
          }
        }
      );
    })
    .query_ve_total_vamp(None, |res| {
      let mut vp =
        Decimal::from_ratio(90_u64 * 2, MAX_LOCK_PERIODS * 10).checked_mul_uint(u(1000)).unwrap();
      adjust_vp_and_slope(&mut vp, 2).unwrap();
      let total_vp = u(1000) + vp;

      assert_eq!(
        res.unwrap(),
        VotingPowerResponse {
          vamp: total_vp * u(2)
        }
      )
    });
}

#[test]
fn test_locks_transfer() {
  let mut suite = TestingSuite::def();

  let user1 = suite.address("user1").to_string();
  let user2 = suite.address("user2").to_string();

  suite
    .init()
    .ve_create_lock_execute(WEEK * 2, native("uluna", 1000u128), "user1", |res| {
      res.assert_attribute(attr("action", "ve/create_lock")).unwrap();
      res.assert_attribute(attr("token_id", "1")).unwrap();
    })
    .ve_create_lock_execute(WEEK * 2, native("uluna", 1000u128), "user2", |res| {
      res.assert_attribute(attr("token_id", "2")).unwrap();
    })
    .query_gauge_user_info(user1.to_string(), Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(172),
          fixed_amount: u(1000),
          slope: u(86),
          gauge_votes: vec![]
        }
      );
    })
    .ve_transfer_nft_execute(user2.clone(), "1".to_string(), "user1", |res| {
      res.assert_attribute(attr("new_owner", user2.clone())).unwrap();
    })
    .query_ve_total_vamp(None, |res| {
      let mut vp =
        Decimal::from_ratio(90_u64 * 2, MAX_LOCK_PERIODS * 10).checked_mul_uint(u(1000)).unwrap();
      adjust_vp_and_slope(&mut vp, 2).unwrap();
      let total_vp = u(1000) + vp;

      assert_eq!(total_vp, u(1172));

      assert_eq!(
        res.unwrap(),
        VotingPowerResponse {
          vamp: total_vp * u(2)
        }
      )
    })
    .query_ve_owner_of("1".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        OwnerOfResponse {
          owner: user2.to_string(),
          approvals: vec![]
        }
      )
    })
    .query_ve_owner_of("2".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        OwnerOfResponse {
          owner: user2.to_string(),
          approvals: vec![]
        }
      )
    })
    .query_gauge_user_info(user2.to_string(), Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(172 * 2),
          fixed_amount: u(1000 * 2),
          slope: u(86 * 2),
          gauge_votes: vec![]
        }
      );
    })
    .query_gauge_user_info(user1.to_string(), Some(Time::Next), |res| {
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
    .add_one_period()
    .query_gauge_user_info(user2.to_string(), Some(Time::Current), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(172 * 2),
          fixed_amount: u(1000 * 2),
          slope: u(86 * 2),
          gauge_votes: vec![]
        }
      );
    })
    .query_gauge_user_info(user1.to_string(), Some(Time::Current), |res| {
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
    .query_gauge_user_info(user2.to_string(), Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(86 * 2),
          fixed_amount: u(1000 * 2),
          slope: u(86 * 2),
          gauge_votes: vec![]
        }
      );
    })
    .query_gauge_user_info(user1.to_string(), Some(Time::Next), |res| {
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
    .add_one_period()
    .query_gauge_user_info(user2.to_string(), Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(0),
          fixed_amount: u(1000 * 2),
          slope: u(0),
          gauge_votes: vec![]
        }
      );
    })
    .query_gauge_user_info(user1.to_string(), Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(0),
          fixed_amount: u(0),
          slope: u(0),
          gauge_votes: vec![]
        }
      );
    });
}
