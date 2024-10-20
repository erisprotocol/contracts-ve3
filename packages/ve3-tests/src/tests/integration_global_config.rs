use cosmwasm_std::Addr;
use cw_ownable::Ownership;
use ve3_global_config::error::ContractError;
use ve3_shared::constants::AT_FREE_BRIBES;

use crate::common::suite::{InitOptions, TestingSuite};

#[test]
fn test_config_default() {
  let mut suite = TestingSuite::def();
  suite.init();
  let addr = suite.addresses.clone();

  suite
    .q_gc_all_addresses(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          ("ASSET_GAUGE".to_string(), addr.ve3_asset_gauge.clone()),
          ("ASSET_STAKING__emission".to_string(), addr.ve3_asset_staking_3.clone()),
          ("ASSET_STAKING__project".to_string(), addr.ve3_asset_staking_2.clone()),
          ("ASSET_STAKING__stable".to_string(), addr.ve3_asset_staking_1.clone()),
          (
            "ASSET_WHITELIST_CONTROLLER".to_string(),
            Addr::unchecked("terra1yrnr2tuwnz9l95886n8d757lc70g7szefm6jzu885kafdusx3f4sg6uhup")
          ),
          ("BRIBE_MANAGER".to_string(), addr.ve3_bribe_manager.clone()),
          (
            "BRIBE_WHITELIST_CONTROLLER".to_string(),
            Addr::unchecked("terra12c28vjamz0tfwjf3h9669zx4fhkpjnfd75vjsw807h38w5qkv7es0v88dv")
          ),
          ("CONNECTOR__emission".to_string(), addr.ve3_connector_emissions.clone()),
          ("CONNECTOR__project".to_string(), addr.ve3_connector_alliance_eris.clone()),
          ("CONNECTOR__stable".to_string(), addr.ve3_connector_alliance_mock.clone()),
          (
            "DELEGATION_CONTROLLER".to_string(),
            Addr::unchecked("terra15ja5gr6saap69dnszyf3zwh28306xw8sefl8yluvsvkcttxh4u5sv2xus6")
          ),
          (
            "FEE_COLLECTOR".to_string(),
            Addr::unchecked("terra1q7440dq4ydqh3x63rdfljq38xmyutjjzzrzhk9r9d8xmeeaxynxqkyqche")
          ),
          (
            "PDT_CONFIG_OWNER".to_string(),
            Addr::unchecked("terra183hc2qrxu2qm6rfar2sdfpdqzjnrtugpsxm23pw8rqksw0suhgsqddn2gx")
          ),
          (
            "PDT_CONTROLLER".to_string(),
            Addr::unchecked("terra1caaz4kfsu5w86mdc7c85a3zelyew2mlctdct7d64ygcc3myu7axqn9fugn")
          ),
          // (
          //   "PDT_VETO_CONFIG_OWNER".to_string(),
          //   Addr::unchecked("terra1pm88ac7093qmynm2u7lw9tsca7786frym623e3289dvh8yktnzss0h5x2c")
          // ),
          (
            "TAKE_RECIPIENT".to_string(),
            Addr::unchecked("terra1lx7v09sx6mwazws6nd4n499ue7z28d7wyst3js6rtcu47fuwnmtqh5r9xl")
          ),
          (
            "TEAM_WALLET".to_string(),
            Addr::unchecked("terra1k5yhkz4lgvtq3gz4d6th0wpf8g9mtgjp4aaj8gzakz2qcl6r5hcqkw69f7")
          ),
          (
            "VE_GUARDIAN".to_string(),
            Addr::unchecked("terra17jpszh4mc0cxvv6enwvm646dtztcs0anwd234n867qf27umcj8cs55507e")
          ),
          ("VOTING_ESCROW".to_string(), addr.ve3_voting_escrow.clone()),
          ("ZAPPER".to_string(), addr.ve3_zapper.clone()) // (
                                                          //   "GAUGE_CONTROLLER".to_string(),
                                                          //   Addr::unchecked("terra1upd8urhe9wz4mpf42gmc4yv0hgrypjqm3a4qh4s6dxm5w90pae7qxwgf8t")
                                                          // )
        ]
      );
    })
    .q_gc_address_list(AT_FREE_BRIBES.to_string(), |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          addr.ve3_asset_staking_1.clone(),
          addr.ve3_asset_staking_2.clone(),
          addr.ve3_asset_staking_3.clone(),
          addr.creator.clone()
        ]
      )
    });
}

#[test]
fn test_config_update_ownership() {
  let mut suite = TestingSuite::def();
  suite.init_no_config(InitOptions::default());
  let addr = suite.addresses.clone();

  suite
    .q_gc_all_addresses(None, None, |res| {
      let vec: Vec<(String, Addr)> = vec![];
      assert_eq!(res.unwrap(), vec);
    })
    .q_gc_ownership(|res| {
      assert_eq!(
        res.unwrap(),
        Ownership {
          owner: Some(addr.creator.to_string()),
          pending_expiry: None,
          pending_owner: None,
        }
      );
    })
    .e_gc_update_ownership(
      cw_ownable::Action::TransferOwnership {
        new_owner: addr.user2.to_string(),
        expiry: None,
      },
      "anyone",
      |res| {
        let res = res.unwrap_err().downcast::<ContractError>().unwrap();
        assert_eq!(res, ContractError::OwnershipError(cw_ownable::OwnershipError::NotOwner));
      },
    )
    .e_gc_update_ownership(
      cw_ownable::Action::TransferOwnership {
        new_owner: addr.user2.to_string(),
        expiry: None,
      },
      "creator",
      |res| {
        res.unwrap();
      },
    )
    .e_gc_update_ownership(cw_ownable::Action::AcceptOwnership {}, "user1", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(res, ContractError::OwnershipError(cw_ownable::OwnershipError::NotPendingOwner));
    })
    .e_gc_update_ownership(cw_ownable::Action::AcceptOwnership {}, "user2", |res| {
      res.unwrap();
    })
    .q_gc_ownership(|res| {
      assert_eq!(
        res.unwrap(),
        Ownership {
          owner: Some(addr.user2.to_string()),
          pending_expiry: None,
          pending_owner: None,
        }
      );
    });
}

#[test]
fn test_config_update_addresses() {
  let mut suite = TestingSuite::def();
  suite.init_no_config(InitOptions::default());
  let addr = suite.addresses.clone();

  suite
    .e_gc_set_addresses(
      vec![("one".to_string(), addr.user1.to_string())],
      vec![("list".to_string(), vec![addr.user1.to_string(), addr.user2.to_string()])],
      "user",
      |res| {
        let res = res.unwrap_err().downcast::<ContractError>().unwrap();
        assert_eq!(res, ContractError::OwnershipError(cw_ownable::OwnershipError::NotOwner))
      },
    )
    .e_gc_set_addresses(
      vec![("one".to_string(), addr.user1.to_string())],
      vec![("list".to_string(), vec![addr.user1.to_string(), addr.user2.to_string()])],
      "creator",
      |res| {
        res.unwrap();
      },
    )
    .q_gc_all_addresses(None, None, |res| {
      assert_eq!(res.unwrap(), vec![("one".to_string(), addr.user1.clone()),]);
    })
    .q_gc_address_list("list".to_string(), |res| {
      assert_eq!(res.unwrap(), vec![addr.user1.to_string(), addr.user2.to_string()])
    })
    .e_gc_set_addresses(
      vec![("two".to_string(), addr.user2.to_string())],
      vec![("list".to_string(), vec![])],
      "creator",
      |res| {
        res.unwrap();
      },
    )
    .q_gc_all_addresses(None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![("one".to_string(), addr.user1.clone()), ("two".to_string(), addr.user2.clone())]
      );
    })
    .q_gc_addresses(vec!["two".to_string(), "one".to_string()], |res| {
      assert_eq!(
        res.unwrap(),
        vec![("two".to_string(), addr.user2.clone()), ("one".to_string(), addr.user1.clone())]
      );
    })
    .q_gc_address_list("list".to_string(), |res| {
      let vec: Vec<Addr> = vec![];
      assert_eq!(res.unwrap(), vec)
    })
    .e_gc_clear_addresses(vec![("one".to_string())], "creator", |res| {
      res.unwrap();
    })
    .q_gc_all_addresses(None, None, |res| {
      assert_eq!(res.unwrap(), vec![("two".to_string(), addr.user2.clone())]);
    })
    .q_gc_address("two".to_string(), |res| {
      assert_eq!(res.unwrap(), ("two".to_string(), addr.user2.clone()));
    })
    .e_gc_clear_lists(vec!["list".to_string(), "unknown".to_string()], "creator", |res| {
      res.unwrap();
    })
    .q_gc_address_list("list".to_string(), |res| {
      assert_eq!(
        res.unwrap_err().to_string(),
        "Generic error: Querier contract error: Not found: address type: list".to_string()
      )
    });
}
