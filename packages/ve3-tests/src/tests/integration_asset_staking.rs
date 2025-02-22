use crate::{
  common::{
    helpers::{native_info, u},
    suite::TestingSuite,
  },
  extensions::app_response_ext::{EventChecker, Valid},
};
use cosmwasm_std::{attr, Decimal};
use cw_asset::{AssetInfo, AssetInfoUnchecked};
use ve3_asset_staking::error::ContractError;
use ve3_shared::{
  constants::AT_ASSET_WHITELIST_CONTROLLER, error::SharedError, msgs_asset_staking::*,
};

#[test]
fn test_add_remove_asset() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .e_staking_whitelist_assets(
      vec![addr.lp_native_info_checked().into(), AssetInfo::cw20(addr.lp_cw20.clone()).into()],
      "user1",
      |res| {
        let res = res.unwrap_err().downcast::<ContractError>().unwrap();
        assert_eq!(
          res,
          SharedError::UnauthorizedMissingRight(
            AT_ASSET_WHITELIST_CONTROLLER.to_string(),
            addr.user1.to_string()
          )
          .into()
        );
      },
    )
    .e_staking_whitelist_assets(
      vec![addr.lp_native_info_checked().into(), AssetInfo::cw20(addr.lp_cw20.clone()).into()],
      "AT_ASSET_WHITELIST_CONTROLLER",
      |res| {
        res.unwrap();
      },
    )
    .q_staking_whitelisted_assets(|res| {
      assert_eq!(
        res.unwrap(),
        vec![AssetInfo::cw20(addr.lp_cw20.clone()), addr.lp_native_info_checked()]
      )
    })
    .e_staking_remove_assets(vec![AssetInfo::cw20(addr.lp_cw20.clone())], "user1", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        SharedError::UnauthorizedMissingRight(
          AT_ASSET_WHITELIST_CONTROLLER.to_string(),
          addr.user1.to_string()
        )
        .into()
      );
    })
    .e_staking_remove_assets(
      vec![AssetInfo::cw20(addr.lp_cw20.clone())],
      "AT_ASSET_WHITELIST_CONTROLLER",
      |res| {
        res.unwrap();
      },
    )
    .q_staking_whitelisted_assets(|res| {
      assert_eq!(res.unwrap(), vec![addr.lp_native_info_checked()])
    });
}

#[test]
fn test_asset_config() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .e_staking_whitelist_assets(
      vec![
        AssetInfoWithConfig::new(
          addr.lp_native_info(),
          Some(AssetConfig {
            yearly_take_rate: Some(Decimal::percent(20)),
            stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
              contract: addr.incentive_mock.to_string(),
              reward_infos: vec![AssetInfoUnchecked::native("astro")],
            },
          }),
        ),
        AssetInfo::cw20(addr.lp_cw20.clone()).into(),
      ],
      "AT_ASSET_WHITELIST_CONTROLLER",
      |res| {
        res.unwrap();
      },
    )
    .e_staking_remove_assets(
      vec![AssetInfo::cw20(addr.lp_cw20.clone())],
      "AT_ASSET_WHITELIST_CONTROLLER",
      |res| {
        res.unwrap();
      },
    )
    .q_staking_whitelisted_asset_details(|res| {
      assert_eq!(
        res.unwrap(),
        vec![
          AssetInfoWithRuntime {
            whitelisted: false,
            info: AssetInfo::cw20(addr.lp_cw20.clone()),
            config: AssetConfigRuntime {
              yearly_take_rate: Decimal::percent(10),
              stake_config: ve3_shared::stake_config::StakeConfig::Default,
              last_taken_s: 0,
              taken: u(0),
              harvested: u(0)
            }
          },
          AssetInfoWithRuntime {
            whitelisted: true,
            info: addr.lp_native_info_checked(),
            config: AssetConfigRuntime {
              yearly_take_rate: Decimal::percent(20),
              stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
                contract: addr.incentive_mock.clone(),
                reward_infos: vec![AssetInfo::native("astro")],
              },
              last_taken_s: 0,
              taken: u(0),
              harvested: u(0)
            }
          },
        ]
      )
    });
}

#[test]
fn test_asset_set_config() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_asset_config_no_staking()
    .e_staking_stake(None, addr.fake_native(123), "user1", |res| {
      res.assert_error(ContractError::AssetNotWhitelisted)
    })
    .e_staking_stake(None, addr.fake_cw20(1100), "user1", |res| {
      res.assert_error(ContractError::AssetNotWhitelisted)
    })
    .def_asset_config_astro(|res| res.assert_error(ContractError::AssetAlreadyWhitelisted))
    .e_staking_stake(None, addr.lp_native(1000), "user1", |res| {
      res.assert_attribute(attr("action", "asset/stake"));
      res.assert_attribute(attr("share", "1000"));
    })
    .e_staking_stake(None, addr.lp_cw20(1100), "user1", |res| {
      res.assert_attribute(attr("action", "asset/stake"));
      res.assert_attribute(attr("share", "1100"));
    })
    .e_staking_update_asset_config(
      AssetInfoWithConfig {
        info: addr.lp_native_info(),
        config: None,
      },
      "user1",
      |res| {
        res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
          AT_ASSET_WHITELIST_CONTROLLER.to_string(),
          addr.user1.to_string(),
        )))
      },
    )
    .e_staking_update_asset_config(
      AssetInfoWithConfig {
        info: addr.lp_native_info(),
        config: Some(AssetConfig {
          yearly_take_rate: Some(Decimal::percent(10)),
          stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
            contract: addr.incentive_mock.to_string(),
            reward_infos: vec![AssetInfoUnchecked::native("astro")],
          },
        }),
      },
      "AT_ASSET_WHITELIST_CONTROLLER",
      |res| {
        res.assert_attribute(attr("action", "asset/update_asset_config"));
        res.assert_attribute(attr("action", "mock/deposit"));
        res.assert_attribute(attr("mock/amount", addr.lp_native_str(1000)));
        res.assert_attribute(attr("action", "asset/track_bribes_callback"));
      },
    )
    .e_staking_update_asset_config(
      AssetInfoWithConfig {
        info: addr.lp_native_info(),
        config: None,
      },
      "AT_ASSET_WHITELIST_CONTROLLER",
      |res| {
        res.assert_attribute(attr("action", "asset/update_asset_config"));
        res.assert_attribute(attr("action", "mock/withdraw"));
        res.assert_attribute(attr("mock/amount", addr.lp_native_str(1000)));
        res.assert_attribute(attr("action", "asset/track_bribes_callback"));
      },
    );
}

#[test]
fn test_asset_take_rate() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();
  let take_recipient = suite.address("AT_TAKE_RECIPIENT");

  suite
    .def_asset_config_astro(|res| {
      res.assert_attribute(attr("action", "asset/whitelist_assets"));
    })
    .e_staking_stake(None, addr.lp_native(10_000_000), "user1", |res| {
      res.assert_attribute(attr("action", "asset/stake"));
      res.assert_attribute(attr("share", "10000000"));
      res.assert_attribute(attr("action", "mock/deposit"));
      res.assert_attribute(attr("mock/amount", addr.lp_native_str(10000000)));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
    })
    .add_one_period()
    .e_staking_distribute_take_rate(Some(true), None, "user1", |res| {
      res.assert_attribute(attr("action", "asset/distribute_take_rate"));
      res.assert_attribute(attr("take", addr.lp_native_str(19178)));
      res.assert_attribute(attr("action", "mock/withdraw"));
      res.assert_attribute(attr("mock/amount", addr.lp_native_str(19178)));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
      // setup to receive 10000 astro per week
      res.assert_attribute(attr("bribe", "native:astro:10000"));
      res.assert_attribute_ty("transfer", attr("recipient", take_recipient.to_string()));
      // 7 / 365 * 10% * 10_000000 = 19,178
      res.assert_attribute_ty(
        "transfer",
        attr("amount", format!("19178{0}", addr.astroport_luna_usdc_lp)),
      );
    })
    .add_one_period()
    .e_staking_distribute_bribes(Some(true), None, "user1", |res| {
      res.assert_error(ve3_bribe_manager::error::ContractError::AssetNotWhitelisted);
    })
    .e_bribe_whitelist_assets(
      vec![native_info("astro").into()],
      "AT_BRIBE_WHITELIST_CONTROLLER",
      |res| res.assert_valid(),
    )
    .e_staking_distribute_bribes(None, None, "user1", |res| {
      res.assert_attribute(attr("action", "asset/distribute_bribes_callback"));
      res.assert_attribute(attr("action", "bribe/add_bribe"));
      res.assert_attribute(attr("start", "77"));
      res.assert_attribute(attr("end", "77"));
      res.assert_attribute(attr("added", "native:astro:10000"));
    })
    .add_one_period()
    .e_staking_distribute_bribes(Some(true), None, "user1", |res| {
      res.assert_attribute(attr("action", "asset/distribute_bribes"));
      res.assert_attribute(attr("action", "mock/claimrewards"));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
      res.assert_attribute(attr("bribe", "native:astro:20000"));
      res.assert_attribute(attr("action", "bribe/add_bribe"));
      res.assert_attribute(attr("start", "78"));
      res.assert_attribute(attr("end", "78"));
      res.assert_attribute(attr("added", "native:astro:20000"));
    });
}

#[test]
fn test_asset_unstake() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_asset_config_astro(|res| {
      res.assert_attribute(attr("action", "asset/whitelist_assets"));
    })
    .e_staking_stake(None, addr.lp_native(10_000_000), "user1", |res| {
      res.assert_attribute(attr("action", "asset/stake"));
      res.assert_attribute(attr("share", "10000000"));
      res.assert_attribute(attr("action", "mock/deposit"));
      res.assert_attribute(attr("mock/amount", addr.lp_native_str(10000000)));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
    })
    .q_staking_all_staked_balances(
      AllStakedBalancesQuery {
        address: addr.user1.to_string(),
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![StakedBalanceRes {
            asset: addr.lp_native(10000000),
            shares: u(10000000),
            total_shares: u(10000000),
            config: AssetConfigRuntime {
              last_taken_s: 1712242800,
              taken: u(0),
              harvested: u(0),
              yearly_take_rate: Decimal::percent(10),
              stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
                contract: addr.incentive_mock.clone(),
                reward_infos: vec![native_info("astro")]
              }
            }
          }]
        )
      },
    )
    // taken: 19178
    .add_one_period()
    .q_staking_all_staked_balances(
      AllStakedBalancesQuery {
        address: addr.user1.to_string(),
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![StakedBalanceRes {
            asset: addr.lp_native(9980822),
            shares: u(10000000),
            total_shares: u(10000000),
            config: AssetConfigRuntime {
              last_taken_s: 1712847600,
              taken: u(19178),
              harvested: u(0),
              yearly_take_rate: Decimal::percent(10),
              stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
                contract: addr.incentive_mock.clone(),
                reward_infos: vec![native_info("astro")]
              }
            }
          }]
        )
      },
    )
    .q_staking_staked_balance(
      AssetQuery {
        address: addr.user1.to_string(),
        asset: addr.lp_native_info_checked(),
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          StakedBalanceRes {
            asset: addr.lp_native(9980822),
            shares: u(10000000),
            total_shares: u(10000000),
            config: AssetConfigRuntime {
              last_taken_s: 1712847600,
              taken: u(19178),
              harvested: u(0),
              yearly_take_rate: Decimal::percent(10),
              stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
                contract: addr.incentive_mock.clone(),
                reward_infos: vec![native_info("astro")]
              }
            }
          }
        )
      },
    )
    .e_staking_unstake(addr.lp_native(1000), "user2", |res| {
      res.assert_error(ContractError::AmountCannotBeZero {})
    })
    .e_staking_unstake(addr.lp_native(1000000), "user1", |res| {
      res.assert_attribute(attr("action", "asset/unstake"));
      res.assert_attribute(attr("amount", "999999"));
      res.assert_attribute(attr("share", "1001921"));
      res.assert_attribute(attr("action", "mock/withdraw"));
      res.assert_attribute(attr("mock/amount", addr.lp_native_str(999999)));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
      res.assert_attribute(attr("bribe", "native:astro:10000"));
      res.assert_attribute_ty("transfer", attr("recipient", addr.user1.to_string()));
      res.assert_attribute_ty(
        "transfer",
        attr("amount", format!("999999{0}", addr.astroport_luna_usdc_lp)),
      );
    })
    .e_staking_unstake(addr.lp_native(10000000), "user1", |res| {
      res.assert_attribute(attr("action", "asset/unstake"));
      // 8980822 + 1000000 = 9980822 (taken: 19178 - see previous test)
      res.assert_attribute(attr("amount", "8980823"));
      res.assert_attribute(attr("share", "8998079"));
      res.assert_attribute(attr("action", "mock/withdraw"));
      res.assert_attribute(attr("mock/amount", addr.lp_native_str(8980823)));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
      res.assert_attribute_ty("transfer", attr("recipient", addr.user1.to_string()));
      res.assert_attribute_ty(
        "transfer",
        attr("amount", format!("8980823{0}", addr.astroport_luna_usdc_lp)),
      );
    })
    .q_staking_all_staked_balances(
      AllStakedBalancesQuery {
        address: addr.user1.to_string(),
      },
      |res| assert_eq!(res.unwrap(), vec![]),
    )
    .add_one_period()
    .e_staking_distribute_take_rate(None, None, "user1", |res| res.assert_valid())
    .q_staking_all_staked_balances(
      AllStakedBalancesQuery {
        address: addr.user1.to_string(),
      },
      |res| assert_eq!(res.unwrap(), vec![]),
    );
}

#[test]
fn test_asset_unstake_recipient() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_asset_config_astro(|res| {
      res.assert_attribute(attr("action", "asset/whitelist_assets"));
    })
    .e_staking_stake(None, addr.lp_native(10_000_000), "user1", |res| {
      res.assert_attribute(attr("action", "asset/stake"));
      res.assert_attribute(attr("share", "10000000"));
      res.assert_attribute(attr("action", "mock/deposit"));
      res.assert_attribute(attr("mock/amount", addr.lp_native_str(10000000)));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
    })
    .e_staking_unstake_recipient(addr.lp_native(1000000), "user1", "user2", |res| {
      res.assert_attribute(attr("action", "asset/unstake"));
      res.assert_attribute(attr("amount", "1000000"));
      res.assert_attribute(attr("share", "1000000"));
      res.assert_attribute(attr("action", "mock/withdraw"));
      res.assert_attribute(attr("mock/amount", addr.lp_native_str(1000000)));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
      res.assert_attribute_ty("transfer", attr("recipient", addr.user2.to_string()));
      res.assert_attribute_ty(
        "transfer",
        attr("amount", format!("1000000{0}", addr.astroport_luna_usdc_lp)),
      );
    })
    .e_staking_unstake(addr.lp_native(9000000), "user1", |res| {
      res.assert_attribute(attr("action", "asset/unstake"));
      res.assert_attribute(attr("amount", "9000000"));
      res.assert_attribute(attr("share", "9000000"));
      res.assert_attribute(attr("action", "mock/withdraw"));
      res.assert_attribute(attr("mock/amount", addr.lp_native_str(9000000)));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
      res.assert_attribute_ty("transfer", attr("recipient", addr.user1.to_string()));
      res.assert_attribute_ty(
        "transfer",
        attr("amount", format!("9000000{0}", addr.astroport_luna_usdc_lp)),
      );
    })
    .q_staking_all_staked_balances(
      AllStakedBalancesQuery {
        address: addr.user1.to_string(),
      },
      |res| assert_eq!(res.unwrap(), vec![]),
    )
    .e_staking_unstake(addr.lp_native(100), "user1", |res| {
      res.assert_error(ContractError::AmountCannotBeZero {})
    });
}

#[test]
fn test_asset_second_user() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_asset_config_astro(|res| res.assert_valid())
    .e_staking_stake(None, addr.lp_native(10_000_000), "user1", |res| res.assert_valid())
    .add_one_period()
    .e_staking_stake(None, addr.lp_native(1_000_000), "user2", |res| {
      res.assert_attribute(attr("action", "asset/stake"));
      res.assert_attribute(attr("amount", "1000000"));
      res.assert_attribute(attr("share", "1001921"));
      res.assert_attribute(attr("action", "mock/deposit"));
      res.assert_attribute(attr("mock/amount", addr.lp_native_str(1000000)));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
      res.assert_attribute(attr("bribe", "native:astro:10000"));
    })
    .q_staking_all_staked_balances(
      AllStakedBalancesQuery {
        address: addr.user2.to_string(),
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![StakedBalanceRes {
            asset: addr.lp_native(999999),
            shares: u(1001921),
            total_shares: u(11001921),
            config: AssetConfigRuntime {
              last_taken_s: 1712847600,
              taken: u(19178),
              harvested: u(0),
              yearly_take_rate: Decimal::percent(10),
              stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
                contract: addr.incentive_mock.clone(),
                reward_infos: vec![native_info("astro")]
              }
            }
          }]
        )
      },
    )
    .q_staking_pool_stakers(
      PoolStakersQuery {
        asset: addr.lp_native_info_checked(),
        start_after: None,
        limit: None,
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![
            UserStakedBalanceRes {
              user: addr.user1.clone(),
              shares: u(10000000),
              balance: u(9998253)
            },
            UserStakedBalanceRes {
              user: addr.user2.clone(),
              shares: u(1001921),
              balance: u(1001746)
            }
          ]
        )
      },
    )
    .e_staking_unstake(addr.lp_native(1_000_000), "user2", |res| {
      res.assert_attribute(attr("action", "asset/unstake"));
      res.assert_attribute(attr("amount", "999999"));
      res.assert_attribute(attr("share", "1001921"));
    });
}

#[test]
fn test_asset_recipient() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_asset_config_astro(|res| res.assert_valid())
    .e_staking_stake(None, addr.lp_native(10_000_000), "user1", |res| res.assert_valid())
    .add_one_period()
    .e_staking_stake(Some("user1"), addr.lp_native(1_000_000), "user2", |res| {
      res.assert_attribute(attr("action", "asset/stake"));
      res.assert_attribute(attr("amount", "1000000"));
      res.assert_attribute(attr("share", "1001921"));
      res.assert_attribute(attr("action", "mock/deposit"));
      res.assert_attribute(attr("mock/amount", addr.lp_native_str(1000000)));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
      res.assert_attribute(attr("bribe", "native:astro:10000"));
    })
    .q_staking_all_staked_balances(
      AllStakedBalancesQuery {
        address: addr.user1.to_string(),
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![StakedBalanceRes {
            asset: addr.lp_native(10980822),
            shares: u(11001921),
            total_shares: u(11001921),
            config: AssetConfigRuntime {
              last_taken_s: 1712847600,
              taken: u(19178),
              harvested: u(0),
              yearly_take_rate: Decimal::percent(10),
              stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
                contract: addr.incentive_mock.clone(),
                reward_infos: vec![native_info("astro")]
              }
            }
          }]
        )
      },
    )
    .e_staking_unstake(addr.lp_native(1_000_000), "user2", |res| {
      res.assert_error(ContractError::AmountCannotBeZero {})
    })
    .q_staking_total_staked_balances(|res| {
      assert_eq!(
        res.unwrap(),
        vec![StakedBalanceRes {
          // taken: 19178
          asset: addr.lp_native(10980822),
          shares: u(11001921),
          total_shares: u(11001921),
          config: AssetConfigRuntime {
            last_taken_s: 1712847600,
            taken: u(19178),
            harvested: u(0),
            yearly_take_rate: Decimal::percent(10),
            stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
              contract: addr.incentive_mock.clone(),
              reward_infos: vec![native_info("astro")]
            }
          }
        }]
      )
    })
    .e_staking_unstake(addr.lp_native(11_000_000), "user1", |res| {
      res.assert_attribute(attr("action", "asset/unstake"));
      // 19178 + 10980822 = 11000000
      res.assert_attribute(attr("amount", "10980822"));
      res.assert_attribute(attr("share", "11001921"));
      res.assert_attribute(attr("action", "mock/withdraw"));
      res.assert_attribute(attr("mock/amount", addr.lp_native_str(10980822)));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
    })
    .q_staking_total_staked_balances(|res| {
      assert_eq!(
        res.unwrap(),
        vec![StakedBalanceRes {
          // taken: 19178
          asset: addr.lp_native(0),
          shares: u(0),
          total_shares: u(0),
          config: AssetConfigRuntime {
            last_taken_s: 1712847600,
            taken: u(19178),
            harvested: u(0),
            yearly_take_rate: Decimal::percent(10),
            stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
              contract: addr.incentive_mock.clone(),
              reward_infos: vec![native_info("astro")]
            }
          }
        }]
      )
    })
    .add_periods(10)
    .e_staking_stake(None, addr.lp_native(1_000_000), "user2", |res| res.assert_valid())
    .q_staking_all_staked_balances(
      AllStakedBalancesQuery {
        address: addr.user2.to_string(),
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![StakedBalanceRes {
            asset: addr.lp_native(1_000_000),
            shares: u(1000000),
            total_shares: u(1000000),
            config: AssetConfigRuntime {
              last_taken_s: 1718895600,
              taken: u(19178),
              harvested: u(0),
              yearly_take_rate: Decimal::percent(10),
              stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
                contract: addr.incentive_mock.clone(),
                reward_infos: vec![native_info("astro")]
              }
            }
          }]
        )
      },
    )
    .q_staking_total_staked_balances(|res| {
      assert_eq!(
        res.unwrap(),
        vec![StakedBalanceRes {
          asset: addr.lp_native(1_000_000),
          shares: u(1000000),
          total_shares: u(1000000),
          config: AssetConfigRuntime {
            last_taken_s: 1718895600,
            taken: u(19178),
            harvested: u(0),
            yearly_take_rate: Decimal::percent(10),
            stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
              contract: addr.incentive_mock.clone(),
              reward_infos: vec![native_info("astro")]
            }
          }
        }]
      )
    })
    .q_staking_pool_stakers(
      PoolStakersQuery {
        asset: addr.lp_native_info_checked(),
        start_after: None,
        limit: None,
      },
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![UserStakedBalanceRes {
            user: addr.user2.clone(),
            shares: u(1000000),
            balance: u(1019178)
          }]
        )
      },
    );
}
