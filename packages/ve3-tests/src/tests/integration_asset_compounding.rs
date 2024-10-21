use std::str::FromStr;

use crate::{
  common::{
    helpers::{u, Uint128},
    suite::TestingSuite,
  },
  extensions::app_response_ext::{EventChecker, Valid},
};
use cosmwasm_std::{attr, Decimal};
use ve3_asset_compounding::error::ContractError;
use ve3_shared::{
  adapters::asset_staking::AssetStaking,
  constants::AT_BOT,
  error::SharedError,
  extensions::{asset_ext::AssetExt, asset_info_ext::AssetInfoExt},
  msgs_asset_compounding::{
    CompoundingAssetConfig, Config, ExchangeHistory, ExchangeRatesResponse, UserInfoResponse,
  },
};

#[test]
fn test_compounding_setup() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    // rewards: 3 for native : 1 for cw20
    .def_setup_staking()
    .e_compound_initialize_asset(addr.lp_cw20_info(), &addr.gauge_2, "anyone", vec![], |res| {
      res.assert_error(ContractError::SharedError(ve3_shared::error::SharedError::WrongDeposit(
        "expected 1 coins".to_string(),
      )))
    })
    .e_compound_initialize_asset(
      addr.lp_cw20_info(),
      &addr.gauge_2,
      "user1",
      vec![addr.usdc(10).to_coin().unwrap()],
      |res| {
        res.assert_error(ContractError::SharedError(ve3_shared::error::SharedError::WrongDeposit(
          "expected 10000000uluna coins".to_string(),
        )))
      },
    )
    .e_compound_initialize_asset(
      addr.ampluna_info(),
      &addr.gauge_2,
      "user1",
      vec![addr.uluna(10000000).to_coin().unwrap()],
      |res| {
        res.assert_error(ContractError::AssetNotWhitelisted(
          addr.gauge_2.to_string(),
          addr.ampluna_info_checked().to_string(),
        ));
      },
    )
    .e_compound_initialize_asset(
      addr.lp_cw20_info(),
      &addr.gauge_1,
      "user1",
      vec![addr.uluna(10000000).to_coin().unwrap()],
      |res| {
        res.assert_error(ContractError::AssetNotWhitelisted(
          addr.gauge_1.to_string(),
          addr.lp_cw20_info_checked().to_string(),
        ));
      },
    )
    .e_compound_initialize_asset(
      addr.lp_cw20_info(),
      &addr.gauge_2,
      "user1",
      vec![addr.uluna(10000000).to_coin().unwrap()],
      |res| {
        res.assert_attribute(attr("action", "asset-compounding/initialize_asset"));
        res.assert_attribute(attr(
          "amplp",
          format!("factory/{0}/0/{1}/amplp", addr.ve3_asset_compounding, addr.gauge_2),
        ));
        res.assert_attribute(attr("staking", addr.ve3_asset_staking_2.to_string()));
      },
    )
    .q_compound_asset_config(addr.lp_cw20_info_checked(), &addr.gauge_2, |res| {
      assert_eq!(
        res.unwrap(),
        CompoundingAssetConfig {
          asset_info: addr.lp_cw20_info_checked(),
          gauge: addr.gauge_2.to_string(),
          staking: AssetStaking(addr.ve3_asset_staking_2.clone()),
          amp_denom: format!("factory/{0}/0/{1}/amplp", addr.ve3_asset_compounding, addr.gauge_2),
          total_bond_share: Uint128(0),
          zasset_denom: addr.zasset_denom.to_string(),
          reward_asset_info: addr.ampluna_info_checked(),
          fee: None
        }
      )
    })
    .e_compound_initialize_asset(
      addr.lp_native_info(),
      &addr.gauge_2,
      "user1",
      vec![addr.uluna(10000000).to_coin().unwrap()],
      |res| {
        res.assert_attribute(attr("action", "asset-compounding/initialize_asset"));
        res.assert_attribute(attr(
          "amplp",
          format!("factory/{0}/1/{1}/amplp", addr.ve3_asset_compounding, addr.gauge_2),
        ));
        res.assert_attribute(attr("staking", addr.ve3_asset_staking_2.to_string()));
      },
    )
    .q_compound_asset_configs(None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          CompoundingAssetConfig {
            asset_info: addr.lp_cw20_info_checked(),
            gauge: addr.gauge_2.to_string(),
            staking: AssetStaking(addr.ve3_asset_staking_2.clone()),
            amp_denom: format!("factory/{0}/0/{1}/amplp", addr.ve3_asset_compounding, addr.gauge_2),
            total_bond_share: Uint128(0),
            zasset_denom: addr.zasset_denom.to_string(),
            reward_asset_info: addr.ampluna_info_checked(),
            fee: None
          },
          CompoundingAssetConfig {
            asset_info: addr.lp_native_info_checked(),
            gauge: addr.gauge_2.to_string(),
            staking: AssetStaking(addr.ve3_asset_staking_2.clone()),
            amp_denom: format!("factory/{0}/1/{1}/amplp", addr.ve3_asset_compounding, addr.gauge_2),
            total_bond_share: Uint128(0),
            zasset_denom: addr.zasset_denom.to_string(),
            reward_asset_info: addr.ampluna_info_checked(),
            fee: None
          }
        ]
      )
    })
    .e_compound_update_config(
      None,
      None,
      None,
      None,
      Some(vec![(addr.gauge_2.clone(), addr.ampluna_info(), Some(Decimal::percent(20)))]),
      "anyone",
      |res| res.assert_error(ContractError::SharedError(SharedError::Unauthorized {})),
    )
    .e_compound_update_config(
      None,
      None,
      None,
      None,
      Some(vec![(addr.gauge_2.clone(), addr.ampluna_info(), Some(Decimal::percent(20)))]),
      "creator",
      |res| {
        res.assert_error(ContractError::AssetNotWhitelisted(
          addr.gauge_2.to_string(),
          addr.ampluna_info_checked().to_string(),
        ))
      },
    )
    .e_compound_update_config(
      None,
      None,
      None,
      None,
      Some(vec![(addr.gauge_2.clone(), addr.lp_native_info(), Some(Decimal::percent(20)))]),
      "creator",
      |res| {
        res.assert_valid();
      },
    )
    .q_compound_asset_configs(None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          CompoundingAssetConfig {
            asset_info: addr.lp_cw20_info_checked(),
            gauge: addr.gauge_2.to_string(),
            staking: AssetStaking(addr.ve3_asset_staking_2.clone()),
            amp_denom: format!("factory/{0}/0/{1}/amplp", addr.ve3_asset_compounding, addr.gauge_2),
            total_bond_share: Uint128(0),
            zasset_denom: addr.zasset_denom.to_string(),
            reward_asset_info: addr.ampluna_info_checked(),
            fee: None
          },
          CompoundingAssetConfig {
            asset_info: addr.lp_native_info_checked(),
            gauge: addr.gauge_2.to_string(),
            staking: AssetStaking(addr.ve3_asset_staking_2.clone()),
            amp_denom: format!("factory/{0}/1/{1}/amplp", addr.ve3_asset_compounding, addr.gauge_2),
            total_bond_share: Uint128(0),
            zasset_denom: addr.zasset_denom.to_string(),
            reward_asset_info: addr.ampluna_info_checked(),
            fee: Some(Decimal::percent(20))
          }
        ]
      )
    })
    .q_compound_asset_configs(
      Some(vec![(addr.gauge_2.to_string(), addr.lp_native_info_checked())]),
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![CompoundingAssetConfig {
            asset_info: addr.lp_native_info_checked(),
            gauge: addr.gauge_2.to_string(),
            staking: AssetStaking(addr.ve3_asset_staking_2.clone()),
            amp_denom: format!("factory/{0}/1/{1}/amplp", addr.ve3_asset_compounding, addr.gauge_2),
            total_bond_share: Uint128(0),
            zasset_denom: addr.zasset_denom.to_string(),
            reward_asset_info: addr.ampluna_info_checked(),
            fee: Some(Decimal::percent(20))
          }]
        )
      },
    );
}

#[test]
fn test_compounding_config() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    // rewards: 3 for native : 1 for cw20
    .def_setup_staking()
    .q_compound_config(|res| {
      assert_eq!(
        res.unwrap(),
        Config {
          global_config_addr: addr.ve3_global_config.clone(),
          fee: Decimal::percent(10),
          fee_collector: addr.fee.clone(),
          deposit_profit_delay_s: 100,
          denom_creation_fee: addr.uluna(10_000000)
        }
      )
    })
    .e_compound_update_config(
      Some(Decimal::percent(30)),
      Some(addr.user1.to_string()),
      Some(0),
      Some(addr.uluna(2).into()),
      None,
      "anyone",
      |res| res.assert_error(ContractError::SharedError(SharedError::Unauthorized {})),
    )
    .e_compound_update_config(
      Some(Decimal::percent(30)),
      Some(addr.user1.to_string()),
      Some(0),
      Some(addr.uluna(2).into()),
      None,
      "creator",
      |res| res.assert_error(ContractError::ConfigValueTooHigh("fee".to_string())),
    )
    .e_compound_update_config(
      Some(Decimal::percent(20)),
      Some(addr.user1.to_string()),
      Some(0),
      Some(addr.uluna(2).into()),
      None,
      "creator",
      |res| res.assert_valid(),
    )
    .q_compound_config(|res| {
      assert_eq!(
        res.unwrap(),
        Config {
          global_config_addr: addr.ve3_global_config.clone(),
          fee: Decimal::percent(20),
          fee_collector: addr.user1.clone(),
          deposit_profit_delay_s: 0,
          denom_creation_fee: addr.uluna(2)
        }
      )
    });
}

#[test]
fn test_compounding_stake_unstake() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_setup_compounding()
    .e_compound_stake(None, &addr.gauge_2, addr.uluna(100), "user1", |res| {
      res.assert_error(ContractError::AssetNotWhitelisted(
        addr.gauge_2.to_string(),
        addr.uluna_info_checked().to_string(),
      ))
    })
    .e_compound_stake(None, &addr.gauge_2, addr.ampluna(100), "user1", |res| {
      res.assert_error(ContractError::AssetNotWhitelisted(
        addr.gauge_2.to_string(),
        addr.ampluna_info_checked().to_string(),
      ))
    })
    .e_compound_stake(None, &addr.gauge_2, addr.lp_native(100), "user1", |res| {
      res.assert_attribute(attr("action", "asset-compounding/stake"));
      res.assert_attribute(attr("user", addr.user1.to_string()));
      res.assert_attribute(attr("asset", addr.lp_native_info_checked().to_string()));
      res.assert_attribute(attr("gauge", addr.gauge_2.to_string()));
      res.assert_attribute(attr("bond_amount", "100"));

      res.assert_attribute(attr("action", "asset/stake"));
      res.assert_attribute(attr("user", addr.ve3_asset_compounding.to_string()));

      res.assert_transfer(addr.user1.to_string(), addr.amplp1(100));
    })
    .e_compound_unstake(None, "user1", vec![], |res| {
      res.assert_error(ContractError::OnlySingleAssetAllowed {});
    })
    .e_compound_unstake(None, "user1", vec![addr.uluna(100).to_coin().unwrap()], |res| {
      res.assert_error(ContractError::AmplpNotFound("uluna".to_string()));
    })
    .e_compound_unstake(None, "user1", vec![addr.amplp1(100).to_coin().unwrap()], |res| {
      res.assert_attribute(attr("action", "asset-compounding/unstake"));
      res.assert_attribute(attr("user", addr.user1.to_string()));
      res.assert_attribute(attr("returned", addr.lp_native(100).to_string()));

      res.assert_attribute(attr("action", "asset/unstake"));
      res.assert_attribute(attr("user", addr.ve3_asset_compounding.to_string()));
      res.assert_attribute(attr("recipient", addr.user1.to_string()));

      res.assert_transfer(addr.user1.to_string(), addr.lp_native(100));
    });
}
#[test]
fn test_compounding_stake_unstake_cw20() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_setup_compounding()
    .e_compound_stake(None, &addr.gauge_2, addr.lp_cw20(100), "user1", |res| {
      res.assert_attribute(attr("action", "asset-compounding/stake"));
      res.assert_attribute(attr("user", addr.user1.to_string()));
      res.assert_attribute(attr("asset", addr.lp_cw20_info_checked().to_string()));
      res.assert_attribute(attr("gauge", addr.gauge_2.to_string()));
      res.assert_attribute(attr("bond_amount", "100"));

      res.assert_attribute(attr("action", "asset/stake"));
      res.assert_attribute(attr("user", addr.ve3_asset_compounding.to_string()));

      res.assert_transfer(addr.user1.to_string(), addr.amplp0(100));
    })
    .e_compound_unstake(None, "user1", vec![addr.amplp0(100).to_coin().unwrap()], |res| {
      res.assert_attribute(attr("action", "asset-compounding/unstake"));
      res.assert_attribute(attr("user", addr.user1.to_string()));
      res.assert_attribute(attr("returned", addr.lp_cw20(100).to_string()));

      res.assert_attribute(attr("action", "asset/unstake"));
      res.assert_attribute(attr("user", addr.ve3_asset_compounding.to_string()));
      res.assert_attribute(attr("recipient", addr.user1.to_string()));

      res.assert_transfer(addr.user1.to_string(), addr.lp_cw20(100));
    });
}

#[test]
fn test_compounding_user_infos() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_setup_compounding()
    .e_compound_stake(None, &addr.gauge_2, addr.lp_native(100), "user1", |res| {
      res.assert_valid();
    })
    .e_compound_stake(None, &addr.gauge_2, addr.lp_cw20(10), "user1", |res| {
      res.assert_valid();
    })
    .e_compound_stake(None, &addr.gauge_2, addr.lp_cw20(100), "user2", |res| {
      res.assert_valid();
    })
    .q_compound_user_infos(None, "user1", |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          UserInfoResponse {
            gauge: addr.gauge_2.to_string(),
            asset: addr.lp_cw20_info_checked(),
            total_lp: u(110),
            total_amplp: u(110),
            user_lp: u(10),
            user_amplp: u(10),
          },
          UserInfoResponse {
            gauge: addr.gauge_2.to_string(),
            asset: addr.lp_native_info_checked(),
            total_lp: u(100),
            total_amplp: u(100),
            user_lp: u(100),
            user_amplp: u(100),
          }
        ]
      );
    })
    .q_compound_user_infos(
      Some(vec![(addr.gauge_1.to_string(), addr.lp_cw20_info_checked())]),
      "user2",
      |res| {
        assert_eq!(res.unwrap_err().to_string(), "Generic error: Querier contract error: asset not whitelisted in gauge: stable, asset: cw20:terra1zwv6feuzhy6a9wekh96cd57lsarmqlwxdypdsplw6zhfncqw6ftqynf7kp".to_string());
      },
    )
    .q_compound_user_infos(
      Some(vec![(addr.gauge_2.to_string(), addr.lp_native_info_checked()), (addr.gauge_2.to_string(), addr.lp_cw20_info_checked())]),
      "user2",
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![
            UserInfoResponse {
              gauge: addr.gauge_2.to_string(),
              asset: addr.lp_native_info_checked(),
              total_lp: u(100),
              total_amplp: u(100),
              user_lp: u(0),
              user_amplp: u(0),
            },
            UserInfoResponse {
              gauge: addr.gauge_2.to_string(),
              asset: addr.lp_cw20_info_checked(),
              total_lp: u(110),
              total_amplp: u(110),
              user_lp: u(100),
              user_amplp: u(100),
            }
          ]
        );
      },
    );
}

#[test]
fn test_compounding_compound() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .def_setup_compounding()
    .def_setup_zapper()
    .e_compound_stake(None, &addr.gauge_2, addr.lp_native(100_000000), "user1", |res| {
      res.assert_valid();
    })
    .e_compound_stake(None, &addr.gauge_2, addr.lp_cw20(10_000000), "user1", |res| {
      res.assert_valid();
    })
    .e_compound_stake(None, &addr.gauge_2, addr.lp_cw20(100_000000), "user2", |res| {
      res.assert_valid();
    })
    .q_compound_user_infos(None, "user1", |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          UserInfoResponse {
            gauge: addr.gauge_2.to_string(),
            asset: addr.lp_cw20_info_checked(),
            total_lp: u(110_000000),
            total_amplp: u(110_000000),
            user_lp: u(10_000000),
            user_amplp: u(10_000000),
          },
          UserInfoResponse {
            gauge: addr.gauge_2.to_string(),
            asset: addr.lp_native_info_checked(),
            total_lp: u(100_000000),
            total_amplp: u(100_000000),
            user_lp: u(100_000000),
            user_amplp: u(100_000000),
          }
        ]
      );
    })
    .def_add_staking_rewards(100_000000)
    .e_compound_compound(None, addr.lp_native_info(), &addr.gauge_2, "user1", |res| {
      res.assert_error(ContractError::SharedError(SharedError::UnauthorizedMissingRight(
        AT_BOT.to_string(),
        addr.user1.to_string(),
      )))
    })
    .e_compound_compound(Some(u(17225379)), addr.lp_native_info(), &addr.gauge_2, "dca1", |res| {
      assert_eq!(
        res.unwrap_err().root_cause().to_string(),
        "AssertionFailed: balance 17225378 smaller than expected 17225379".to_string()
      )
    })
    .e_compound_compound(Some(u(17223946)), addr.lp_native_info(), &addr.gauge_2, "dca1", |res| {
      res.assert_attribute(attr("action", "asset-compounding/callback_track_exchange_rate"));
      res.assert_attribute(attr("exchange_rate", "1.17225378"));
      res.assert_attribute(attr("assets", "29845154uluna, 9941781ibc/usdc"));
      res.assert_attribute(attr("amount", "17225378"));
      res.assert_attribute(attr("fee", "5555555"));
      res.assert_transfer(addr.fee.clone(), addr.ampluna(5555555));
    })
    .q_compound_user_infos(None, "user1", |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          UserInfoResponse {
            gauge: addr.gauge_2.to_string(),
            asset: addr.lp_cw20_info_checked(),
            total_lp: u(110_000000),
            total_amplp: u(110_000000),
            user_lp: u(10_000000),
            user_amplp: u(10_000000),
          },
          UserInfoResponse {
            gauge: addr.gauge_2.to_string(),
            asset: addr.lp_native_info_checked(),
            total_lp: u(100_000000 + 17225378),
            total_amplp: u(100_000000),
            user_lp: u(100_000000 + 17225378),
            user_amplp: u(100_000000),
          }
        ]
      );
    })
    .e_compound_stake(None, &addr.gauge_2, addr.lp_native(100_000000), "user2", |res| {
      res.assert_attribute(attr("action", "asset-compounding/stake"));
      res.assert_attribute(attr("user", addr.user2.to_string()));
      res.assert_attribute(attr("gauge", addr.gauge_2.to_string()));
      res.assert_attribute(attr("bond_amount", "100000000"));
      res.assert_attribute(attr("bond_share_adjusted", "85305760"));
      res.assert_attribute(attr("bond_share", "85305760"));
    })
    .q_compound_user_infos(
      Some(vec![(addr.gauge_2.clone(), addr.lp_native_info_checked())]),
      "user2",
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![UserInfoResponse {
            gauge: addr.gauge_2.to_string(),
            asset: addr.lp_native_info_checked(),
            total_lp: u(217225378),
            total_amplp: u(100_000000 + 85305760),
            user_lp: u(99_999999),
            user_amplp: u(85305760),
          }]
        );
      },
    )
    // dropping LP
    .add_periods(10)
    .q_compound_user_infos(
      Some(vec![(addr.gauge_2.clone(), addr.lp_native_info_checked())]),
      "user2",
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![UserInfoResponse {
            gauge: addr.gauge_2.to_string(),
            asset: addr.lp_native_info_checked(),
            total_lp: u(208893446),
            total_amplp: u(185305760),
            user_lp: u(96164383),
            user_amplp: u(85305760),
          }]
        );
      },
    )
    .e_compound_stake(None, &addr.gauge_2, addr.lp_native(100_000000), "user2", |res| {
      // getting more bond now due to take rate changing the ratio
      res.assert_attribute(attr("action", "asset-compounding/stake"));
      res.assert_attribute(attr("user", addr.user2.to_string()));
      res.assert_attribute(attr("gauge", addr.gauge_2.to_string()));
      res.assert_attribute(attr("bond_amount", "100000000"));
      res.assert_attribute(attr("bond_share_adjusted", "88708268"));
      res.assert_attribute(attr("bond_share", "88708268"));
    })
    .q_compound_user_infos(
      Some(vec![(addr.gauge_2.clone(), addr.lp_native_info_checked())]),
      "user2",
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![UserInfoResponse {
            gauge: addr.gauge_2.to_string(),
            asset: addr.lp_native_info_checked(),
            total_lp: u(208893446 + 100_000000),
            total_amplp: u(185305760 + 88708268),
            user_lp: u(96164383 + 100_000000),
            user_amplp: u(85305760 + 88708268),
          }]
        );
      },
    )
    .q_compound_user_infos(None, "user1", |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          UserInfoResponse {
            gauge: addr.gauge_2.to_string(),
            asset: addr.lp_cw20_info_checked(),
            total_lp: u(107890411),
            total_amplp: u(110000000),
            user_lp: u(9808219),
            user_amplp: u(10000000),
          },
          UserInfoResponse {
            gauge: addr.gauge_2.to_string(),
            asset: addr.lp_native_info_checked(),
            total_lp: u(308893446),
            total_amplp: u(274014028),
            user_lp: u(112729062),
            user_amplp: u(100000000),
          }
        ]
      );
    })
    .e_compound_compound(None, addr.lp_native_info(), &addr.gauge_2, "dca1", |res| {
      res.assert_error(ContractError::NoRewards);
    })
    .def_add_staking_rewards(100_000000)
    .e_compound_compound(None, addr.lp_native_info(), &addr.gauge_2, "dca1", |res| {
      res.assert_attribute(attr("action", "asset-compounding/callback_track_exchange_rate"));
      res.assert_attribute(attr("exchange_rate", "1.190028475476445315"));
      res.assert_attribute(attr("amount", "17191050"));
      res.assert_attribute(attr("assets", "29795559uluna, 9918688ibc/usdc"));
    })
    .q_compound_exchange_rates(None, None, None, |res| {
      assert_eq!(
        res.unwrap(),
        vec![
          ExchangeRatesResponse {
            gauge: addr.gauge_2.to_string(),
            asset: addr.lp_cw20_info_checked(),
            exchange_rates: vec![],
            apr: None
          },
          ExchangeRatesResponse {
            gauge: addr.gauge_2.to_string(),

            asset: addr.lp_native_info_checked(),
            exchange_rates: vec![
              (
                19894,
                ExchangeHistory {
                  exchange_rate: Decimal::from_str("1.190028475476445315").unwrap(),
                  time_s: 1718895600
                }
              ),
              (
                19824,
                ExchangeHistory {
                  exchange_rate: Decimal::from_str("1.17225378").unwrap(),
                  time_s: 1712847600
                }
              )
            ],
            apr: Some(Decimal::from_str("0.000216611987458957").unwrap())
          }
        ]
      );
    })
    .q_compound_exchange_rates(
      Some(vec![(addr.gauge_2.to_string(), addr.lp_native_info_checked())]),
      None,
      Some(1),
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![ExchangeRatesResponse {
            gauge: addr.gauge_2.to_string(),

            asset: addr.lp_native_info_checked(),
            exchange_rates: vec![(
              19894,
              ExchangeHistory {
                exchange_rate: Decimal::from_str("1.190028475476445315").unwrap(),
                time_s: 1718895600
              }
            )],
            apr: None
          }]
        );
      },
    )
    .q_compound_exchange_rates(
      Some(vec![(addr.gauge_2.to_string(), addr.lp_native_info_checked())]),
      Some(19894),
      Some(1),
      |res| {
        assert_eq!(
          res.unwrap(),
          vec![ExchangeRatesResponse {
            gauge: addr.gauge_2.to_string(),
            asset: addr.lp_native_info_checked(),
            exchange_rates: vec![(
              19824,
              ExchangeHistory {
                exchange_rate: Decimal::from_str("1.17225378").unwrap(),
                time_s: 1712847600
              }
            )],
            apr: None
          }]
        );
      },
    )
    .e_compound_stake(None, &addr.gauge_2, addr.lp_native(100_000000), "user2", |res| {
      // getting more bond now due to take rate changing the ratio
      res.assert_attribute(attr("action", "asset-compounding/stake"));
      res.assert_attribute(attr("user", addr.user2.to_string()));
      res.assert_attribute(attr("gauge", addr.gauge_2.to_string()));
      res.assert_attribute(attr("bond_amount", "100000000"));
      res.assert_attribute(attr("bond_share_adjusted", "84031580"));
      res.assert_attribute(attr("bond_share", "84031602"));
      res.assert_transfer(addr.fee.to_string(), addr.amplp1_info_checked().with_balance(u(22)));
    });
}

#[test]
fn test_compounding_double_setup() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite.def_setup_compounding().e_compound_initialize_asset(
    addr.lp_cw20_info(),
    &addr.gauge_2,
    "user1",
    vec![addr.uluna(10000000).to_coin().unwrap()],
    |res| {
      res.assert_error(ContractError::AssetAlreadyInitialized(
        addr.gauge_2.to_string(),
        addr.lp_cw20_info_checked().to_string(),
      ));
    },
  );
}
