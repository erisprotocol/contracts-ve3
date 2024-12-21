use crate::{
  common::{
    helpers::u,
    suite::TestingSuite,
  },
  extensions::app_response_ext::{EventChecker, Valid},
};
use cosmwasm_std::attr;
use cw721::{ NftInfoResponse,  TokensResponse};
use ve3_shared::{
  constants::SECONDS_PER_WEEK, helpers::time::Time, msgs_asset_gauge::UserInfoExtendedResponse, msgs_voting_escrow::*
};
use ve3_voting_escrow::error::ContractError;

#[test]
fn test_ve_migrate() {
  let mut suite = TestingSuite::def();
  let addr = suite.init_options(crate::common::suite::InitOptions {
    rebase_asset: None,
    mock_zapper: Some(false),
  });

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user1", |res| {
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("token_id", "1"));
    })
    .e_ve_migrate_lock("1".to_string(), addr.ampluna_info(), None, "user2", |res| {
      res.assert_error(ContractError::NftError(cw721_base::ContractError::Ownership(
        cw_ownable::OwnershipError::NotOwner,
      )));
    })
    .e_ve_migrate_lock("1".to_string(), addr.usdc_info(), None, "user1", |res| {
      res.assert_error(ContractError::WrongAsset("ibc/usdc".to_string()));
    })
    .e_ve_migrate_lock("1".to_string(), addr.uluna_info(), None, "user1", |res| {
      res.assert_error(ContractError::CannotMigrateToSameToken("1".to_string(), addr.uluna_info_checked().to_string()));
    })
    .q_ve_tokens(addr.user1.to_string(), None, None, |res| {
      assert_eq!(
        res.unwrap(),
        TokensResponse {
          tokens: vec!["1".to_string()]
        }
      )
    })
    .q_gauge_user_info("user1", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          fixed_amount: u(0),
          slope: u(0),
          voting_power: u(0),
          gauge_votes: vec![]
        }
      )
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          fixed_amount: u(1000),
          slope: u(86),
          voting_power: u(172),
          gauge_votes: vec![]
        }
      )
    })
    .e_ve_migrate_lock("1".to_string(), addr.ampluna_info(), None, "user1", |res| {
      res.assert_attribute(attr("action", "ve/migrate_lock"));
      res.assert_attribute(attr("token_id", "1"));
      res.assert_attribute(attr("fixed_power_before", "1000"));
      res.assert_attribute(attr("migrate_amount", "native:uluna:1000"));
      res.assert_attribute(attr("voting_power", "0"));
      res.assert_attribute(attr("fixed_power", "0"));
      res.assert_attribute(attr("lock_end", "76"));

      res.assert_attribute(attr("action", "swap"));
      res.assert_attribute(attr("return_amount", "831"));

      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("voting_power", "172"));
      res.assert_attribute(attr("fixed_power", "997"));
      res.assert_attribute(attr("lock_end", "76"));
      res.assert_attribute(attr("owner", addr.user1.to_string()));
    })
    .q_gauge_user_info("user1", None, |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          fixed_amount: u(0),
          slope: u(0),
          voting_power: u(0),
          gauge_votes: vec![]
        }
      )
    })
    .q_gauge_user_info("user1", Some(Time::Next), |res| {
      assert_eq!(
        res.unwrap(),
        UserInfoExtendedResponse {
          fixed_amount: u(997),
          slope: u(86),
          voting_power: u(172),
          gauge_votes: vec![]
        }
      )
    })
    .q_ve_tokens(addr.user1.to_string(), None, None, |res| {
      assert_eq!(
        res.unwrap(),
        TokensResponse {
          tokens: vec!["2".to_string()]
        }
      )
    })
    .q_ve_lock_info("1", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 74,
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
    .q_ve_lock_info("2", None, |res| {
      let res = res.unwrap();
      assert_eq!(
        res,
        LockInfoResponse {
          owner: addr.user1.clone(),
          from_period: 74,
          asset: addr.ampluna(831),
          underlying_amount: u(997),
          start: 74,
          end: End::Period(76),
          slope: u(86),
          fixed_amount: u(997),
          voting_power: u(172),
          ..res
        }
      );
    })
    .e_ve_withdraw("1", "user1", |res| {
      res.assert_error(ContractError::LockHasNotExpired {});
    })
    .q_ve_nft_info(
      "1".to_string(),
      |res: Result<NftInfoResponse<Metadata>, cosmwasm_std::StdError>| {
        // no nft
        assert_eq!(res.unwrap_err().to_string(), "Generic error: Querier contract error: type: cw721_base::state::TokenInfo<ve3_shared::msgs_voting_escrow::Metadata>; key: [00, 06, 74, 6F, 6B, 65, 6E, 73, 31] not found".to_string());
      },
    )
    .q_ve_nft_info(
      "2".to_string(),
      |res: Result<NftInfoResponse<Metadata>, cosmwasm_std::StdError>| {
        // no nft
        assert_eq!(res.unwrap(), 
        NftInfoResponse {
          token_uri: None,
          extension: Extension {
            image: None,
            description: None,
            name: None,
            attributes: Some(vec![
              Trait {
                display_type: None,
                trait_type: "asset".to_string(),
                value: addr.ampluna(831).to_string()
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
      );
      },
    )
    .e_ve_migrate_lock("2".to_string(), addr.uluna_info(), None, "user1", |res| {
      res.assert_attribute(attr("action", "ve/migrate_lock"));
      res.assert_attribute(attr("token_id", "2"));
      res.assert_attribute(attr("fixed_power_before", "997"));
      res.assert_attribute(attr("migrate_amount", "cw20:terra14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9ssrc8au:831"));
      res.assert_attribute(attr("voting_power", "0"));
      res.assert_attribute(attr("fixed_power", "0"));
      res.assert_attribute(attr("lock_end", "76"));

      res.assert_attribute(attr("action", "swap"));
      res.assert_attribute(attr("return_amount", "995"));

      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("voting_power", "172"));
      res.assert_attribute(attr("fixed_power", "995"));
      res.assert_attribute(attr("lock_end", "76"));
      res.assert_attribute(attr("owner", addr.user1.to_string()));
    })
    ;
}




#[test]
fn test_ve_migrate_withdraw_orig() {
  let mut suite = TestingSuite::def();
  let addr = suite.init_options(crate::common::suite::InitOptions {
    rebase_asset: None,
    mock_zapper: Some(false),
  });

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user1", |res| {
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("token_id", "1"));
    })
    .add_one_period()
    .e_ve_withdraw("1", "user1", |res| res.assert_error(ContractError::LockHasNotExpired {  }))
    .add_one_period()
    .e_ve_withdraw("1", "user1", |res| {
      res.assert_transfer(addr.user1.to_string(), addr.uluna(1000));
    });
}

#[test]
fn test_ve_migrate_withdraw_ampluna() {
  let mut suite = TestingSuite::def();
  let addr = suite.init_options(crate::common::suite::InitOptions {
    rebase_asset: None,
    mock_zapper: Some(false),
  });

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user1", |res| {
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("token_id", "1"));
    })
    .e_ve_migrate_lock("1".to_string(), addr.ampluna_info(), None, "user1", |res| {
      res.assert_attribute(attr("action", "ve/migrate_lock"));
      res.assert_attribute(attr("token_id", "1"));
      res.assert_attribute(attr("fixed_power_before", "1000"));
      res.assert_attribute(attr("migrate_amount", "native:uluna:1000"));
      res.assert_attribute(attr("voting_power", "0"));
      res.assert_attribute(attr("fixed_power", "0"));
      res.assert_attribute(attr("lock_end", "76"));

      res.assert_attribute(attr("action", "swap"));
      res.assert_attribute(attr("return_amount", "831"));

      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("voting_power", "172"));
      res.assert_attribute(attr("fixed_power", "997"));
      res.assert_attribute(attr("lock_end", "76"));
      res.assert_attribute(attr("owner", addr.user1.to_string()));
    })
    .add_one_period()
    .e_ve_withdraw("2", "user1", |res| res.assert_error(ContractError::LockHasNotExpired {  }))
    .e_ve_withdraw("1", "user1", |res| res.assert_error(ContractError::LockHasNotExpired {  }))
    .add_one_period()
    .e_ve_withdraw("2", "user1", |res| {
      res.assert_transfer(addr.user1.to_string(), addr.ampluna(831));
    })
    .e_ve_withdraw("1", "user1", |res| {
      assert_eq!(res.unwrap_err().root_cause().to_string(), "type: cw721_base::state::TokenInfo<ve3_shared::msgs_voting_escrow::Metadata>; key: [00, 06, 74, 6F, 6B, 65, 6E, 73, 31] not found".to_string());
    })

    ;
}


#[test]
fn test_ve_migrate_withdraw_double_convert() {
  let mut suite = TestingSuite::def();
  let addr = suite.init_options(crate::common::suite::InitOptions {
    rebase_asset: None,
    mock_zapper: Some(false),
  });

  suite
    .e_ve_create_lock_time(SECONDS_PER_WEEK * 2, addr.uluna(1000), "user1", |res| {
      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("token_id", "1"));
    })
    .e_ve_migrate_lock("1".to_string(), addr.ampluna_info(), None, "user1", |res| {
      res.assert_attribute(attr("action", "ve/migrate_lock"));
      res.assert_attribute(attr("token_id", "1"));
      res.assert_attribute(attr("fixed_power_before", "1000"));
      res.assert_attribute(attr("migrate_amount", "native:uluna:1000"));
      res.assert_attribute(attr("voting_power", "0"));
      res.assert_attribute(attr("fixed_power", "0"));
      res.assert_attribute(attr("lock_end", "76"));

      res.assert_attribute(attr("action", "swap"));
      res.assert_attribute(attr("return_amount", "831"));

      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("voting_power", "172"));
      res.assert_attribute(attr("fixed_power", "997"));
      res.assert_attribute(attr("lock_end", "76"));
      res.assert_attribute(attr("owner", addr.user1.to_string()));
    })
    .add_one_period()    
    .e_ve_migrate_lock("2".to_string(), addr.uluna_info(), None, "user1", |res| {
      res.assert_attribute(attr("action", "ve/migrate_lock"));
      res.assert_attribute(attr("token_id", "2"));
      res.assert_attribute(attr("fixed_power_before", "997"));
      res.assert_attribute(attr("migrate_amount", "cw20:terra14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9ssrc8au:831"));
      res.assert_attribute(attr("voting_power", "0"));
      res.assert_attribute(attr("fixed_power", "0"));
      res.assert_attribute(attr("lock_end", "76"));

      res.assert_attribute(attr("action", "swap"));
      res.assert_attribute(attr("return_amount", "995"));

      res.assert_attribute(attr("action", "ve/create_lock"));
      res.assert_attribute(attr("voting_power", "86"));
      res.assert_attribute(attr("fixed_power", "995"));
      res.assert_attribute(attr("lock_end", "76"));
      res.assert_attribute(attr("owner", addr.user1.to_string()));
    })
    .e_ve_withdraw("1", "user1", |res| res.assert_error(ContractError::LockHasNotExpired {  }))
    .e_ve_withdraw("2", "user1", |res| res.assert_error(ContractError::LockHasNotExpired {  }))
    .e_ve_withdraw("3", "user1", |res| res.assert_error(ContractError::LockHasNotExpired {  }))
    .add_one_period()
    
    .e_ve_withdraw("1", "user1", |res| {
      assert_eq!(res.unwrap_err().root_cause().to_string(), "type: cw721_base::state::TokenInfo<ve3_shared::msgs_voting_escrow::Metadata>; key: [00, 06, 74, 6F, 6B, 65, 6E, 73, 31] not found".to_string());
    })
    .e_ve_withdraw("2", "user1", |res| {
      assert_eq!(res.unwrap_err().root_cause().to_string(), "type: cw721_base::state::TokenInfo<ve3_shared::msgs_voting_escrow::Metadata>; key: [00, 06, 74, 6F, 6B, 65, 6E, 73, 32] not found".to_string());
    })
    .e_ve_withdraw("3", "user1", |res| {
      res.assert_transfer(addr.user1.to_string(), addr.uluna(995));
    })

    ;
}
