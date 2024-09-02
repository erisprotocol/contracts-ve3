use std::str::FromStr;

use crate::{
  common::{
    helpers::{cw20, native, u},
    suite::TestingSuite,
  },
  extensions::app_response_ext::{EventChecker, Valid},
};
use cosmwasm_std::{attr, Decimal};
use cw721::{AllNftInfoResponse, Approval, NftInfoResponse, OwnerOfResponse, TokensResponse};
use ve3_shared::{
  constants::{MAX_LOCK_PERIODS, SECONDS_PER_WEEK},
  extensions::decimal_ext::DecimalExt,
  helpers::{slope::adjust_vp_and_slope, time::Time},
  msgs_asset_gauge::UserInfoExtendedResponse,
  msgs_voting_escrow::*,
};
use ve3_voting_escrow::error::ContractError;

#[test]
fn test_locks() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let user1 = suite.address("user1").to_string();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user1", |res| {
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("token_id", "1"));
    })
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user2", |res| {
      res.assert_attribute(attr("token_id", "2"));
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
          vp: total_vp * u(2),
          fixed: u(2000),
          voting_power: u(344)
        }
      )
    });
}

#[test]
fn test_locks_transfer() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  let user2 = addr.user2.to_string();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user1", |res| {
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("token_id", "1"));
    })
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user2", |res| {
      res.assert_attribute(attr("token_id", "2"));
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
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
    .e_ve_transfer_nft(user2.clone(), "1".to_string(), "user1", |res| {
      res.assert_attribute(attr("new_owner", user2.clone()));
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
          vp: total_vp * u(2),
          fixed: u(2000),
          voting_power: u(344)
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
    .q_gauge_user_info("user2", Some(Time::Next), |res| {
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
    .add_one_period()
    .q_gauge_user_info("user2", Some(Time::Current), |res| {
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
    .q_gauge_user_info("user1", Some(Time::Current), |res| {
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
    .q_gauge_user_info("user2", Some(Time::Next), |res| {
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
    .add_one_period()
    .q_gauge_user_info("user2", Some(Time::Next), |res| {
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
    });
}

#[test]
fn test_locks_exchange_rate() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user1", |res| {
      res.assert_valid()
    })
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.ampluna(1000), "user2", |res| {
      res.assert_valid()
    })
    .q_ve_total_vamp(Some(Time::Period(300)), |res| {
      assert_eq!(
        res.unwrap(),
        VotingPowerResponse {
          vp: u(2200),
          fixed: u(2200),
          voting_power: u(0)
        }
      );
    })
    .q_gauge_user_info("user2", Some(Time::Next), |res| {
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
                  value: format!("cw20:{0}:1000", addr.eris_hub_cw20_ampluna)
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
  let addr = suite.init();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user1", |res| {
      res.unwrap();
    })
    .e_ve_extend_lock_time(SECONDS_PER_WEEK * 2, "1", "user1", |res| {
      res.unwrap();
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
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
  let addr = suite.init();
  let ampluna = addr.eris_hub_cw20_ampluna.to_string();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.ampluna(1000), "user1", |res| {
      res.unwrap();
    })
    .add_one_period()
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
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
    // TODO UPDATE EXCHANGE RATE
    .def_change_exchange_rate(Decimal::percent(130))
    .e_ve_extend_lock_time(SECONDS_PER_WEEK * 2, "2", "user1", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::LockDoesNotExist("2".to_string()));
    })
    .e_ve_extend_lock_time(SECONDS_PER_WEEK * 2, "1", "user1", |res| {
      res.unwrap();
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        // 1.3 higher
        UserInfoExtendedResponse {
          voting_power: u(336),
          // rounding issue
          fixed_amount: u(1299),
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
  let addr = suite.init();
  let fake = addr.fake_cw20.clone();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, native("xxx", 1000u128), "user1", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::WrongAsset("xxx".into()));
    })
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, cw20(fake.clone(), 1000u128), "user1", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::WrongAsset(format!("cw20:{fake}")));
    })
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user1", |res| {
      res.unwrap();
    })
    // 2 = wrong asset
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.ampluna(1000), "user1", |res| {
      res.unwrap();
    })
    // 3 = wrong end
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 3, addr.uluna(1000), "user1", |res| {
      res.unwrap();
    })
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user1", |res| {
      res.unwrap();
    })
    .add_one_period()
    .e_ve_merge_lock("1", "2", "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_merge_lock("1", "2", "user1", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::LocksNeedSameAssets("1".into(), "2".into()));
    })
    .e_ve_merge_lock("3", "1", "user1", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::LocksNeedSameEnd("3".into(), "1".into()));
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
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
    .q_ve_lock_info("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: addr.uluna(1000),
          underlying_amount: u(1000),
          start: 74,
          end: End::Period(76),
          slope: u(86),
          fixed_amount: u(1000),
          voting_power: u(86),
          ..res
        }
      );
    })
    .q_ve_lock_vamp("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        VotingPowerResponse {
          vp: u(1086),
          fixed: u(1000),
          voting_power: u(86)
        }
      );
    })
    .q_ve_lock_info("4", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: addr.uluna(1000),
          underlying_amount: u(1000),
          start: 74,
          end: End::Period(76),
          slope: u(86),
          fixed_amount: u(1000),
          voting_power: u(86),
          ..res
        }
      );
    })
    .e_ve_merge_lock("1", "4", "user1", |res| {
      res.assert_attribute(attr("action", "burn"));
      res.assert_attribute(attr("token_id", "4"));
      res.assert_attribute(attr("action", "ve/merge_lock"));
      res.assert_attribute(attr("merge", "1,4"));
    })
    // 1 is doubled
    .q_ve_lock_info("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: addr.uluna(2000),
          underlying_amount: u(2000),
          start: 74,
          end: End::Period(76),
          slope: u(172),
          fixed_amount: u(2000),
          voting_power: u(172),
          ..res
        }
      );
    })
    // tokens 4 is empty
    .q_ve_lock_info("4", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: addr.uluna(0),
          underlying_amount: u(0),
          start: 74,
          end: End::Period(76),
          slope: u(0),
          fixed_amount: u(0),
          voting_power: u(0),
          ..res
        }
      );
    })
    // user info not changed
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
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
    .e_ve_extend_lock_time(SECONDS_PER_WEEK, "1", "user1", |res| {
      res.unwrap();
    })
    // 3 can now be merged
    .e_ve_merge_lock("1", "3", "user1", |res| {
      res.unwrap();
    })
    // 1 is doubled
    .q_ve_lock_info("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: addr.uluna(3000),
          underlying_amount: u(3000),
          start: 74,
          end: End::Period(77),
          slope: u(259),
          fixed_amount: u(3000),
          voting_power: u(518),
          ..res
        }
      );
    })
    // tokens 4 is empty
    .q_ve_lock_info("3", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: addr.uluna(0),
          underlying_amount: u(0),
          start: 74,
          end: End::Period(77),
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
  let addr = suite.init();
  let ampluna = suite.addresses.eris_hub_cw20_ampluna.clone();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 10, addr.ampluna(2000), "user1", |res| {
      res.unwrap();
    })
    .add_one_period()
    .e_ve_split_lock("1", u(1000), Some("user2"), "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_approve("user2", "1".into(), None, "user1", |res| {
      res.unwrap();
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
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
    .q_gauge_user_info("user2", Some(Time::Next), |res| {
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
    .e_ve_split_lock("1", u(1000), Some("user2"), "user2", |res| {
      res.assert_attribute(attr("token_id", "2"));
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
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
    .q_gauge_user_info("user2", Some(Time::Next), |res| {
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
    .q_ve_lock_info("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 75,
          asset: addr.ampluna(1000),
          underlying_amount: u(1200),
          start: 74,
          end: End::Period(84),
          slope: u(103),
          fixed_amount: u(1200),
          voting_power: u(927),
          ..res
        }
      );
    })
    .q_ve_lock_info("2", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user2.clone(),
          from_period: 75,
          asset: addr.ampluna(1000),
          underlying_amount: u(1200),
          start: 75,
          end: End::Period(84),
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
  let addr = suite.init();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 10, addr.ampluna(2000), "user1", |res| {
      res.unwrap();
    })
    .add_periods(10)
    .e_ve_withdraw("1", "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_withdraw("1", "user1", |res| {
      res.assert_attribute(attr("action", "transfer"));
      res.assert_attribute(attr("from", addr.ve3_voting_escrow.to_string()));
      res.assert_attribute(attr("to", addr.user1.to_string()));
    });
}

#[test]
fn test_lock_withdraw_native() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();
  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 10, addr.uluna(2000), "user1", |res| {
      res.unwrap();
    })
    .add_periods(10)
    .e_ve_withdraw("1", "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_withdraw("1", "user1", |res| {
      res.assert_attribute_ty("transfer", attr("sender", addr.ve3_voting_escrow.to_string()));
      res.assert_attribute_ty("transfer", attr("recipient", addr.user1.to_string()));
      res.assert_attribute_ty("transfer", attr("amount", "2000uluna"));
    });
}

#[test]
fn test_lock_increase_cw20() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 10, addr.ampluna(2000), "user1", |res| {
      res.unwrap();
    })
    .add_periods(5)
    .e_ve_extend_lock_amount("2", "user2", native("xxx", 100u128), |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::WrongAsset("xxx".to_string()));
    })
    .e_ve_extend_lock_amount("2", "user2", cw20(addr.fake_cw20.clone(), 100u128), |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::WrongAsset(format!("cw20:{0}", addr.fake_cw20)));
    })
    .e_ve_extend_lock_amount("2", "user2", addr.ampluna(100), |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::LockDoesNotExist("2".to_string()));
    })
    .e_ve_extend_lock_amount("1", "user1", addr.ampluna(1000), |res| {
      res.unwrap();
    })
    .add_periods(5)
    .e_ve_withdraw("1", "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_withdraw("1", "user1", |res| {
      res.assert_attribute(attr("action", "transfer"));
      res.assert_attribute(attr("from", addr.ve3_voting_escrow.to_string()));
      res.assert_attribute(attr("to", addr.user1.to_string()));
      res.assert_attribute(attr("amount", "3000"));
    });
}

#[test]
fn test_lock_increase_native() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 10, addr.uluna(2000), "user1", |res| {
      res.unwrap();
    })
    .add_periods(5)
    .e_ve_extend_lock_amount("2", "user2", native("xxx", 100u128), |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::WrongAsset("xxx".to_string()));
    })
    .e_ve_extend_lock_amount("2", "user2", addr.uluna(100), |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::LockDoesNotExist("2".to_string()));
    })
    .e_ve_extend_lock_amount("1", "user1", addr.uluna(1000), |res| {
      res.unwrap();
    })
    .add_periods(5)
    .e_ve_withdraw("1", "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_withdraw("1", "user1", |res| {
      res.assert_attribute_ty("transfer", attr("sender", addr.ve3_voting_escrow.to_string()));
      res.assert_attribute_ty("transfer", attr("recipient", addr.user1.to_string()));
      res.assert_attribute_ty("transfer", attr("amount", "3000uluna"));
    });
}

#[test]
fn test_lock_permanent() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite
    .e_ve_create_lock_time_any(None, addr.ampluna(2000), "user1", |res| {
      res.unwrap();
    })
    .add_periods(5)
    .e_ve_extend_lock_amount("2", "user2", native("xxx", 100u128), |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::WrongAsset("xxx".to_string()));
    })
    .e_ve_extend_lock_amount("2", "user2", cw20(addr.fake_cw20.clone(), 100u128), |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::WrongAsset(format!("cw20:{0}", addr.fake_cw20)));
    })
    .e_ve_extend_lock_amount("2", "user2", addr.ampluna(100), |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::LockDoesNotExist("2".to_string()));
    })
    .e_ve_extend_lock_amount("1", "user1", addr.ampluna(1000), |res| {
      res.unwrap();
    })
    .q_ve_lock_info("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 79,
          asset: addr.ampluna(3000),
          underlying_amount: u(3600),
          start: 74,
          end: End::Permanent,
          slope: u(0),
          fixed_amount: u(3600),
          // 3600 * 9
          voting_power: u(32400),
          coefficient: Decimal::from_str("9").unwrap()
        }
      );
    })
    .add_periods(5)
    .e_ve_withdraw("1", "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::LockHasNotExpired {});
    })
    .q_ve_lock_info("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 84,
          asset: addr.ampluna(3000),
          underlying_amount: u(3600),
          start: 74,
          end: End::Permanent,
          slope: u(0),
          fixed_amount: u(3600),
          // 3600 * 9
          voting_power: u(32400),
          coefficient: Decimal::from_str("9").unwrap()
        }
      );
    })
    .q_gauge_user_info("user1", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(32400),
          fixed_amount: u(3600),
          slope: u(0),
          gauge_votes: vec![]
        }
      )
    })
    // .print_block("creating lock 2")
    .e_ve_create_lock_time_any(Some(SECONDS_PER_WEEK * 10), addr.uluna(4000), "user1", |res| {
      res.unwrap();
    })
    .q_ve_lock_info("2", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 84,
          asset: addr.uluna(4000),
          underlying_amount: u(4000),
          start: 84,
          end: End::Period(94),

          voting_power: u(3460),
          fixed_amount: u(4000),
          slope: u(346),
          ..res
        }
      );
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(35860),
          fixed_amount: u(7600),
          slope: u(346),
          gauge_votes: vec![]
        }
      )
    })
    .add_periods(30)
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(32400),
          fixed_amount: u(7600),
          slope: u(0),
          gauge_votes: vec![]
        }
      )
    })
    // .print_block("text withdraw 2")
    .e_ve_withdraw("2", "user1", |res| {
      res.assert_attribute(attr("action", "burn"));
      res.assert_attribute(attr("token_id", "2"));
      res.assert_attribute_ty("transfer", attr("sender", addr.ve3_voting_escrow.to_string()));
      res.assert_attribute_ty("transfer", attr("recipient", addr.user1.to_string()));
      res.assert_attribute_ty("transfer", attr("amount", "4000uluna"));
    })
    .q_gauge_user_info("user1", Option::Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(32400),
          fixed_amount: u(3600),
          slope: u(0),
          gauge_votes: vec![]
        }
      )
    })
    .e_ve_unlock_permanent("1".to_string(), "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_unlock_permanent("1".to_string(), "user1", |res| {
      res.assert_attribute_ty("wasm-metadata_changed", attr("token_id", "1"));
    })
    .q_ve_lock_info("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 114,
          asset: addr.ampluna(3000),
          underlying_amount: u(3600),
          start: 74,
          end: End::Period(114 + MAX_LOCK_PERIODS),
          slope: u(311),
          fixed_amount: u(3600),
          // 311 * 104
          // difference to expected due to rounding 32400
          voting_power: u(32344),
          coefficient: Decimal::from_str("9").unwrap()
        }
      );
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(32344),
          fixed_amount: u(3600),
          slope: u(311),
          gauge_votes: vec![]
        }
      )
    })
    .add_periods(104)
    .e_ve_withdraw("1", "user1", |res| {
      res.assert_attribute(attr("action", "burn"));
      res.assert_attribute(attr("token_id", "1"));
      res.assert_attribute(attr("from", addr.ve3_voting_escrow.to_string()));
      res.assert_attribute(attr("to", addr.user1.to_string()));
      res.assert_attribute(attr("amount", "3000"));
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
      )
    });
}

#[test]
fn test_lock_make_permanent() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 10, addr.uluna(2000), "user1", |res| {
      res.unwrap();
    })
    .add_periods(5)
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(865),
          fixed_amount: u(2000),
          slope: u(173),
          gauge_votes: vec![]
        }
      )
    })
    .e_ve_lock_permanent("1", "user2", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        ContractError::NftError(cw721_base::ContractError::Ownership(
          cw_ownable::OwnershipError::NotOwner
        ))
      );
    })
    .e_ve_lock_permanent("1", "user1", |res| {
      res.assert_attribute(attr("action", "ve/lock_permanent"));
      res.assert_attribute(attr("lock_end", "permanent"));
      res.assert_attribute(attr("fixed_power", "2000"));
      res.assert_attribute(attr("voting_power", "18000"));
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(18000),
          fixed_amount: u(2000),
          slope: u(0),
          gauge_votes: vec![]
        }
      )
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
                  value: "permanent".to_string()
                }
              ])
            }
          }
        }
      );
    });
}

#[test]
fn test_lock_merge_permanent() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 10, addr.uluna(  2000), "user1", |res| {
      res.unwrap();
    })
    .add_periods(5)
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 10, addr.uluna(  2000), "user1", |res| {
      res.unwrap();
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(2595),
          fixed_amount: u(4000),
          slope: u(346),
          gauge_votes: vec![]
        }
      )
    })
    .add_one_period()
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(2595 - 346),
          fixed_amount: u(4000),
          slope: u(346),
          gauge_votes: vec![]
        }
      )
    })
    .e_ve_lock_permanent("1", "user1", |res| {
      res.unwrap();
    })
    .e_ve_lock_permanent("2", "user1", |res| {
      res.unwrap();
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(36000),
          fixed_amount: u(4000),
          slope: u(0),
          gauge_votes: vec![]
        }
      )
    })
    .e_ve_merge_lock("1", "2", "user1", |res| {
      res.unwrap();
    })
    .q_ve_all_nft_info("2".to_string(), None, |res| {
      let res = res.unwrap_err();
      assert_eq!(res.to_string(), "Generic error: Querier contract error: type: cw721_base::state::TokenInfo<ve3_shared::msgs_voting_escrow::Metadata>; key: [00, 06, 74, 6F, 6B, 65, 6E, 73, 32] not found".to_string());
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          voting_power: u(36000),
          fixed_amount: u(4000),
          slope: u(0),
          gauge_votes: vec![]
        }
      )
    });
}

#[test]
fn test_config() {
  let mut suite = TestingSuite::def();
  let addr = suite.init();

  suite.q_ve_config(|res| {
    assert_eq!(
      res.unwrap(),
      Config {
        global_config_addr: addr.ve3_global_config.clone(),
        deposit_assets: vec![
          DepositAsset {
            info: addr.uluna_info_checked(),
            config: AssetInfoConfig::Default
          },
          DepositAsset {
            info: addr.ampluna_info_checked(),
            config: AssetInfoConfig::ExchangeRate {
              contract: addr.eris_hub.clone()
            }
          },
        ],
        push_update_contracts: vec![addr.ve3_asset_gauge.clone()],
        decommissioned: None
      }
    )
  });
}
