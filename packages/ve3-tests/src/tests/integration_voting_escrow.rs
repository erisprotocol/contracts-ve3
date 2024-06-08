use std::str::FromStr;

use crate::{
  common::{
    helpers::{cw20, native, u},
    suite::TestingSuite,
  },
  extensions::app_response_ext::EventChecker,
};
use cosmwasm_std::{attr, Decimal};
use cw721::{AllNftInfoResponse, Approval, NftInfoResponse, OwnerOfResponse, TokensResponse};
use ve3_shared::{
  constants::{MAX_LOCK_PERIODS, WEEK},
  extensions::decimal_ext::DecimalExt,
  helpers::{slope::adjust_vp_and_slope, time::Time},
  msgs_asset_gauge::UserInfoExtendedResponse,
  msgs_voting_escrow::*,
};
use ve3_voting_escrow::error::ContractError;

#[test]
fn test_locks() {
  let mut suite = TestingSuite::def();

  let user1 = suite.address("user1").to_string();

  suite
    .init()
    .e_ve_create_lock_execute(WEEK * 2, native("uluna", 1000u128), "user1", |res| {
      res.assert_attribute(attr("action", "ve/create_lock")).unwrap();
      res.assert_attribute(attr("token_id", "1")).unwrap();
    })
    .e_ve_create_lock_execute(WEEK * 2, native("uluna", 1000u128), "user2", |res| {
      res.assert_attribute(attr("token_id", "2")).unwrap();
    })
    .q_ve_all_tokens(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        TokensResponse {
          tokens: vec!["1".to_string(), "2".to_string()]
        }
      )
    })
    .q_ve_all_nft_info("1".to_string(), None, |res| {
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
    .q_ve_total_vamp(None, |res| {
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
  let addr = suite.addresses.clone();

  let user1 = addr.user1.to_string();
  let user2 = addr.user2.to_string();

  suite
    .init()
    .e_ve_create_lock_execute(WEEK * 2, native("uluna", 1000u128), "user1", |res| {
      res.assert_attribute(attr("action", "ve/create_lock")).unwrap();
      res.assert_attribute(attr("token_id", "1")).unwrap();
    })
    .e_ve_create_lock_execute(WEEK * 2, native("uluna", 1000u128), "user2", |res| {
      res.assert_attribute(attr("token_id", "2")).unwrap();
    })
    .q_gauge_user_info(user1.to_string(), Some(Time::Next), |res| {
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
    .e_ve_transfer_nft_execute(user2.clone(), "1".to_string(), "user1", |res| {
      res.assert_attribute(attr("new_owner", user2.clone())).unwrap();
    })
    .q_ve_total_vamp(None, |res| {
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
    .q_ve_owner_of("1".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        OwnerOfResponse {
          owner: user2.to_string(),
          approvals: vec![]
        }
      )
    })
    .q_ve_owner_of("2".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        OwnerOfResponse {
          owner: user2.to_string(),
          approvals: vec![]
        }
      )
    })
    .q_gauge_user_info(user2.to_string(), Some(Time::Next), |res| {
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
    .q_gauge_user_info(user1.to_string(), Some(Time::Next), |res| {
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
    .q_gauge_user_info(user2.to_string(), Some(Time::Current), |res| {
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
    .q_gauge_user_info(user1.to_string(), Some(Time::Current), |res| {
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
    .q_gauge_user_info(user2.to_string(), Some(Time::Next), |res| {
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
    .q_gauge_user_info(user1.to_string(), Some(Time::Next), |res| {
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
    .q_gauge_user_info(user2.to_string(), Some(Time::Next), |res| {
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
    .q_gauge_user_info(user1.to_string(), Some(Time::Next), |res| {
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

#[test]
fn test_locks_exchange_rate() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();

  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_execute(WEEK * 2, native("uluna", 1000u128), "user1", |res| {
      res.unwrap();
    })
    .e_ve_create_lock_execute(WEEK * 2, addr.ampluna(1000u128), "user2", |res| {
      res.unwrap();
    })
    .q_gauge_user_info(addr.user2.to_string(), Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        // 1.2 higher
        UserInfoExtendedResponse {
          voting_power: u(206),
          fixed_amount: u(1200),
          slope: u(103),
          gauge_votes: vec![]
        }
      );
    })
    .q_ve_all_nft_info("2".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        AllNftInfoResponse::<Extension> {
          access: OwnerOfResponse {
            owner: addr.user2.to_string(),
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
                  value: format!("cw20:{0}:1000", addr.eris_hub_cw20)
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
    });
}

#[test]
fn test_locks_lock_extension() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_execute(WEEK * 2, native("uluna", 1000u128), "user1", |res| {
      res.unwrap();
    })
    .e_ve_extend_lock_time_execute(WEEK * 2, "1", "user1", |res| {
      res.unwrap();
    })
    .q_gauge_user_info(addr.user1.to_string(), Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        // 1.2 higher
        UserInfoExtendedResponse {
          voting_power: u(344),
          fixed_amount: u(1000),
          slope: u(86),
          gauge_votes: vec![]
        }
      );
    })
    .q_ve_all_nft_info("1".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        AllNftInfoResponse::<Extension> {
          access: OwnerOfResponse {
            owner: addr.user1.to_string(),
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
                  value: "78".to_string()
                }
              ])
            }
          }
        }
      );
    });
}

#[test]
fn test_locks_lock_extension_ampluna() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();
  let ampluna = addr.eris_hub_cw20.to_string();

  suite
    .e_ve_create_lock_execute(WEEK * 2, addr.ampluna(1000u128), "user1", |res| {
      res.unwrap();
    })
    .add_one_period()
    .q_gauge_user_info(addr.user1.to_string(), Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        // 1.3 higher
        UserInfoExtendedResponse {
          voting_power: u(103),
          fixed_amount: u(1200),
          slope: u(103),
          gauge_votes: vec![]
        }
      );
    })
    .e_hub_update_exchange_rate(Decimal::from_str("1.3").unwrap(), "creator", |res| {
      res.unwrap();
    })
    .e_ve_extend_lock_time_execute(WEEK * 2, "2", "user1", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::LockDoesNotExist("2".to_string()));
    })
    .e_ve_extend_lock_time_execute(WEEK * 2, "1", "user1", |res| {
      res.unwrap();
    })
    .q_gauge_user_info(addr.user1.to_string(), Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        // 1.3 higher
        UserInfoExtendedResponse {
          voting_power: u(336),
          fixed_amount: u(1300),
          slope: u(112),
          gauge_votes: vec![]
        }
      );
    })
    .q_ve_all_nft_info("1".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        AllNftInfoResponse::<Extension> {
          access: OwnerOfResponse {
            owner: addr.user1.to_string(),
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
                  value: format!("cw20:{ampluna}:1000")
                },
                Trait {
                  display_type: None,
                  trait_type: "start".to_string(),
                  value: "74".to_string()
                },
                Trait {
                  display_type: None,
                  trait_type: "end".to_string(),
                  value: "78".to_string()
                }
              ])
            }
          }
        }
      );
    });
}

#[test]
fn test_locks_merge() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();
  let fake = addr.fake_cw20.clone();

  suite
    .e_ve_create_lock_execute(WEEK * 2, native("xxx", 1000u128), "user1", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::WrongAsset("xxx".into()));
    })
    .e_ve_create_lock_execute(WEEK * 2, cw20(fake.clone(), 1000u128), "user1", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::WrongAsset(format!("cw20:{fake}")));
    })
    .e_ve_create_lock_execute(WEEK * 2, native("uluna", 1000u128), "user1", |res| {
      res.unwrap();
    })
    // 2 = wrong asset
    .e_ve_create_lock_execute(WEEK * 2, addr.ampluna(1000u128), "user1", |res| {
      res.unwrap();
    })
    // 3 = wrong end
    .e_ve_create_lock_execute(WEEK * 3, native("uluna", 1000u128), "user1", |res| {
      res.unwrap();
    })
    .e_ve_create_lock_execute(WEEK * 2, native("uluna", 1000u128), "user1", |res| {
      res.unwrap();
    })
    .add_one_period()
    .e_ve_merge_lock_execute("1", "2", "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_merge_lock_execute("1", "2", "user1", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::LocksNeedSameAssets("1".into(), "2".into()));
    })
    .e_ve_merge_lock_execute("3", "1", "user1", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::LocksNeedSameEnd("3".into(), "1".into()));
    })
    .q_gauge_user_info(addr.user1.to_string(), Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        // 1.3 higher
        UserInfoExtendedResponse {
          voting_power: u(447),
          fixed_amount: u(4200),
          slope: u(361),
          gauge_votes: vec![]
        }
      );
    })
    // tokens 1 and 4 exist
    .q_ve_lock_info("1".to_string(), None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: native("uluna", 1000u128),
          underlying_amount: u(1000),
          start: 74,
          end: 76,
          slope: u(86),
          fixed_amount: u(1000),
          voting_power: u(86),
          ..res
        }
      );
    })
    .q_ve_lock_info("4".to_string(), None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: native("uluna", 1000u128),
          underlying_amount: u(1000),
          start: 74,
          end: 76,
          slope: u(86),
          fixed_amount: u(1000),
          voting_power: u(86),
          ..res
        }
      );
    })
    .e_ve_merge_lock_execute("1", "4", "user1", |res| {
      res.assert_attribute(attr("action", "burn")).unwrap();
      res.assert_attribute(attr("token_id", "4")).unwrap();
      res.assert_attribute(attr("action", "ve/merge_lock")).unwrap();
      res.assert_attribute(attr("merge", "1,4")).unwrap();
    })
    // 1 is doubled
    .q_ve_lock_info("1".to_string(), None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: native("uluna", 2000u128),
          underlying_amount: u(2000),
          start: 74,
          end: 76,
          slope: u(172),
          fixed_amount: u(2000),
          voting_power: u(172),
          ..res
        }
      );
    })
    // tokens 4 is empty
    .q_ve_lock_info("4".to_string(), None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: native("uluna", 0u128),
          underlying_amount: u(0),
          start: 74,
          end: 76,
          slope: u(0),
          fixed_amount: u(0),
          voting_power: u(0),
          ..res
        }
      );
    })
    // user info not changed
    .q_gauge_user_info(addr.user1.to_string(), Some(Time::Next), |res| {
      // merge doesnt change amount for user (in this case)
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(447),
          fixed_amount: u(4200),
          slope: u(361),
          gauge_votes: vec![]
        }
      );
    })
    .q_ve_all_nft_info("1".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        AllNftInfoResponse::<Extension> {
          access: OwnerOfResponse {
            owner: addr.user1.to_string(),
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
                  value: "native:uluna:2000".to_string()
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
    .e_ve_extend_lock_time_execute(WEEK, "1", "user1", |res| {
      res.unwrap();
    })
    // 3 can now be merged
    .e_ve_merge_lock_execute("1", "3", "user1", |res| {
      res.unwrap();
    })
    // 1 is doubled
    .q_ve_lock_info("1".to_string(), None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: native("uluna", 3000u128),
          underlying_amount: u(3000),
          start: 74,
          end: 77,
          slope: u(259),
          fixed_amount: u(3000),
          voting_power: u(518),
          ..res
        }
      );
    })
    // tokens 4 is empty
    .q_ve_lock_info("3".to_string(), None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: native("uluna", 0u128),
          underlying_amount: u(0),
          start: 74,
          end: 77,
          slope: u(0),
          fixed_amount: u(0),
          voting_power: u(0),
          ..res
        }
      );
    })
    .q_ve_all_nft_info("1".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        AllNftInfoResponse::<Extension> {
          access: OwnerOfResponse {
            owner: addr.user1.to_string(),
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
                  value: "native:uluna:3000".to_string()
                },
                Trait {
                  display_type: None,
                  trait_type: "start".to_string(),
                  value: "74".to_string()
                },
                Trait {
                  display_type: None,
                  trait_type: "end".to_string(),
                  value: "77".to_string()
                }
              ])
            }
          }
        }
      );
    });
}

#[test]
fn test_locks_split() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();
  let ampluna = suite.addresses.eris_hub_cw20.clone();

  suite
    .e_ve_create_lock_execute(WEEK * 10, addr.ampluna(2000u128), "user1", |res| {
      res.unwrap();
    })
    .add_one_period()
    .e_ve_split_lock_execute("1", u(1000), Some("user2"), "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_approve_execute("user2", "1".into(), None, "user1", |res| {
      res.unwrap();
    })
    .q_gauge_user_info(addr.user1.to_string(), Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(1863),
          fixed_amount: u(2400),
          slope: u(207),
          gauge_votes: vec![]
        }
      );
    })
    .q_gauge_user_info(addr.user2.to_string(), Some(Time::Next), |res| {
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
    .e_ve_split_lock_execute("1", u(1000), Some("user2"), "user2", |res| {
      res.assert_attribute(attr("token_id", "2")).unwrap();
    })
    .q_gauge_user_info(addr.user1.to_string(), Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(927),
          fixed_amount: u(1200),
          slope: u(103),
          gauge_votes: vec![]
        }
      );
    })
    .q_gauge_user_info(addr.user2.to_string(), Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(927),
          fixed_amount: u(1200),
          slope: u(103),
          gauge_votes: vec![]
        }
      );
    })
    .q_ve_all_nft_info("1".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        AllNftInfoResponse::<Extension> {
          access: OwnerOfResponse {
            owner: addr.user1.to_string(),
            approvals: vec![Approval {
              spender: addr.user2.to_string(),
              expires: cw20::Expiration::Never {}
            }]
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
                  value: format!("cw20:{ampluna}:1000")
                },
                Trait {
                  display_type: None,
                  trait_type: "start".to_string(),
                  value: "74".to_string()
                },
                Trait {
                  display_type: None,
                  trait_type: "end".to_string(),
                  value: "84".to_string()
                }
              ])
            }
          }
        }
      );
    })
    .q_ve_all_nft_info("2".to_string(), None, |res| {
      assert_eq!(
        res.unwrap(),
        AllNftInfoResponse::<Extension> {
          access: OwnerOfResponse {
            owner: addr.user2.to_string(),
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
                  value: format!("cw20:{ampluna}:1000")
                },
                Trait {
                  display_type: None,
                  trait_type: "start".to_string(),
                  value: "75".to_string()
                },
                Trait {
                  display_type: None,
                  trait_type: "end".to_string(),
                  value: "84".to_string()
                }
              ])
            }
          }
        }
      );
    })
    .q_ve_lock_info("1".to_string(), None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: addr.ampluna(1000u128),
          underlying_amount: u(1200),
          start: 74,
          end: 84,
          slope: u(103),
          fixed_amount: u(1200),
          voting_power: u(927),
          ..res
        }
      );
    })
    .q_ve_lock_info("2".to_string(), None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user2.clone(),
          from_period: 75,
          asset: addr.ampluna(1000u128),
          underlying_amount: u(1200),
          start: 75,
          end: 84,
          slope: u(103),
          fixed_amount: u(1200),
          voting_power: u(927),
          ..res
        }
      );
    });
}

#[test]
fn test_lock_withdraw_cw20() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_execute(WEEK * 10, addr.ampluna(2000u128), "user1", |res| {
      res.unwrap();
    })
    .add_periods(10)
    .e_ve_withdraw_execute("1", "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_withdraw_execute("1", "user1", |res| {
      res.assert_attribute(attr("action", "transfer")).unwrap();
      res.assert_attribute(attr("from", addr.ve3_voting_escrow.to_string())).unwrap();
      res.assert_attribute(attr("to", addr.user1.to_string())).unwrap();
    });
}

#[test]
fn test_lock_withdraw_native() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_execute(WEEK * 10, native("uluna", 2000u128), "user1", |res| {
      res.unwrap();
    })
    .add_periods(10)
    .e_ve_withdraw_execute("1", "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_withdraw_execute("1", "user1", |res| {
      res
        .assert_attribute_ty("transfer", attr("sender", addr.ve3_voting_escrow.to_string()))
        .unwrap();
      res.assert_attribute_ty("transfer", attr("recipient", addr.user1.to_string())).unwrap();
      res.assert_attribute_ty("transfer", attr("amount", "2000uluna")).unwrap();
    });
}

#[test]
fn test_lock_increase_cw20() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_execute(WEEK * 10, addr.ampluna(2000u128), "user1", |res| {
      res.unwrap();
    })
    .add_periods(5)
    .e_ve_extend_lock_amount_execute("2", "user2", native("xxx", 100u128), |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::WrongAsset("xxx".to_string()));
    })
    .e_ve_extend_lock_amount_execute("2", "user2", cw20(addr.fake_cw20.clone(), 100u128), |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::WrongAsset(format!("cw20:{0}", addr.fake_cw20)));
    })
    .e_ve_extend_lock_amount_execute("2", "user2", addr.ampluna(100u128), |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::LockDoesNotExist("2".to_string()));
    })
    .e_ve_extend_lock_amount_execute("1", "user1", addr.ampluna(1000u128), |res| {
      res.unwrap();
    })
    .add_periods(5)
    .e_ve_withdraw_execute("1", "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_withdraw_execute("1", "user1", |res| {
      res.assert_attribute(attr("action", "transfer")).unwrap();
      res.assert_attribute(attr("from", addr.ve3_voting_escrow.to_string())).unwrap();
      res.assert_attribute(attr("to", addr.user1.to_string())).unwrap();
      res.assert_attribute(attr("amount", "3000")).unwrap();
    });
}

#[test]
fn test_lock_increase_native() {
  let mut suite = TestingSuite::def();
  let suite = suite.init();
  let addr = suite.addresses.clone();

  suite
    .e_ve_create_lock_execute(WEEK * 10, native("uluna", 2000u128), "user1", |res| {
      res.unwrap();
    })
    .add_periods(5)
    .e_ve_extend_lock_amount_execute("2", "user2", native("xxx", 100u128), |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::WrongAsset("xxx".to_string()));
    })
    .e_ve_extend_lock_amount_execute("2", "user2", native("uluna", 100u128), |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::LockDoesNotExist("2".to_string()));
    })
    .e_ve_extend_lock_amount_execute("1", "user1", native("uluna", 1000u128), |res| {
      res.unwrap();
    })
    .add_periods(5)
    .e_ve_withdraw_execute("1", "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_withdraw_execute("1", "user1", |res| {
      res
        .assert_attribute_ty("transfer", attr("sender", addr.ve3_voting_escrow.to_string()))
        .unwrap();
      res.assert_attribute_ty("transfer", attr("recipient", addr.user1.to_string())).unwrap();
      res.assert_attribute_ty("transfer", attr("amount", "3000uluna")).unwrap();
    });
}
