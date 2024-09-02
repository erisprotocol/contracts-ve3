use super::helpers::{u, Addr};
use super::suite::{InitOptions, TestingSuite};
use crate::common::suite_contracts::*;
use crate::mocks::{alliance_rewards_mock, astroport_pair_mock, incentive_mock};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20Coin;
use cw_asset::{AssetInfoBase, AssetInfoUnchecked, AssetUnchecked};
use cw_multi_test::Executor;
use eris::hub::DelegationStrategy;
use serde::Serialize;
use std::vec;
use ve3_shared::extensions::asset_ext::AssetExt;
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;
use ve3_shared::helpers::assets::Assets;
use ve3_shared::msgs_asset_gauge::GaugeConfig;
use ve3_shared::msgs_voting_escrow::DepositAsset;
use ve3_shared::{msgs_connector_alliance, msgs_connector_emission, msgs_phoenix_treasury};

impl TestingSuite {
  #[track_caller]
  pub(super) fn init_contract<T: Serialize>(&mut self, code_id: u64, msg: T, name: &str) -> Addr {
    let creator = self.creator().clone();
    self
      .app
      .instantiate_contract(
        code_id,
        creator.clone(),
        &msg,
        &[],
        name.to_string(),
        Some(creator.to_string()),
      )
      .unwrap()
  }

  pub(super) fn create_global_config(&mut self) {
    let code_id = self.app.store_code(ve3_global_config());

    let msg = ve3_shared::msgs_global_config::InstantiateMsg {
      owner: self.creator().to_string(),
    };

    self.addresses.ve3_global_config = self.init_contract(code_id, msg, "ve3_global_config");
  }

  pub(super) fn create_asset_gauge(&mut self, init: InitOptions) {
    let code_id = self.app.store_code(ve3_asset_gauge());

    let rebase_asset = init.rebase_asset.unwrap_or(self.addresses.ampluna_info());

    let msg = ve3_shared::msgs_asset_gauge::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      rebase_asset,
      gauges: vec![
        GaugeConfig {
          name: self.gauge1(),
          min_gauge_percentage: Decimal::percent(10),
        },
        GaugeConfig {
          name: self.gauge2(),
          min_gauge_percentage: Decimal::percent(0),
        },
        GaugeConfig {
          name: self.gauge3(),
          min_gauge_percentage: Decimal::percent(0),
        },
      ],
    };

    self.addresses.ve3_asset_gauge = self.init_contract(code_id, msg, "ve3_asset_gauge");
  }

  pub(super) fn create_asset_staking_1(&mut self) {
    let code_id = self.app.store_code(ve3_asset_staking());

    let msg = ve3_shared::msgs_asset_staking::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      reward_info: AssetInfoBase::Native("uluna".to_string()),
      default_yearly_take_rate: Decimal::percent(10),
      gauge: self.gauge1(),
    };

    self.addresses.ve3_asset_staking_1 = self.init_contract(code_id, msg, "ve3_asset_staking_1");
  }

  pub(super) fn create_asset_staking_2(&mut self) {
    let code_id = self.app.store_code(ve3_asset_staking());

    let addr = self.addresses.clone();

    let msg = ve3_shared::msgs_asset_staking::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      reward_info: addr.zasset_info(),
      default_yearly_take_rate: Decimal::percent(10),
      gauge: self.gauge2(),
    };

    self.addresses.ve3_asset_staking_2 = self.init_contract(code_id, msg, "create_asset_staking_2");
  }

  pub(super) fn create_asset_staking_3(&mut self) {
    let code_id = self.app.store_code(ve3_asset_staking());

    let addr = self.addresses.clone();

    let msg = ve3_shared::msgs_asset_staking::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      reward_info: addr.uluna_info(),
      default_yearly_take_rate: Decimal::percent(10),
      gauge: self.gauge3(),
    };

    self.addresses.ve3_asset_staking_3 = self.init_contract(code_id, msg, "create_asset_staking_3");
  }

  pub(super) fn create_bribe_manager(&mut self) {
    let code_id = self.app.store_code(ve3_bribe_manager());

    let ampluna = self.addresses.ampluna_info();
    let msg = ve3_shared::msgs_bribe_manager::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      whitelist: vec![AssetInfoUnchecked::native("uluna"), ampluna],
      fee: AssetUnchecked::native("uluna", 10_000000u128),
    };

    self.addresses.ve3_bribe_manager = self.init_contract(code_id, msg, "ve3_bribe_manager");
  }

  pub(super) fn create_connector_alliance_eris(&mut self) {
    let code_id = self.app.store_code(ve3_connector_alliance());
    let msg = msgs_connector_alliance::InstantiateMsg {
      alliance_token_denom: "vt".to_string(),
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      gauge: self.addresses.gauge_2.clone(),
      reward_denom: "uluna".to_string(),
      zasset_denom: "zluna".to_string(),
      lst_hub_address: self.addresses.eris_hub.to_string(),
      lst_asset_info: AssetInfoUnchecked::cw20(self.addresses.eris_hub_cw20_ampluna.to_string()),
    };

    self.addresses.ve3_connector_alliance_eris =
      self.init_contract(code_id, msg, "ve3_connector_alliance_eris");

    let config: msgs_connector_alliance::Config = self
      .app
      .wrap()
      .query_wasm_smart(
        self.addresses.ve3_connector_alliance_eris.clone(),
        &msgs_connector_alliance::QueryMsg::Config {},
      )
      .unwrap();

    self.addresses.zasset_denom = config.zasset_denom;
  }

  pub(super) fn create_connector_emissions(&mut self) {
    let code_id = self.app.store_code(ve3_connector_emission());
    let msg = msgs_connector_emission::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      gauge: self.addresses.gauge_3.clone(),
      emission_token: AssetInfoBase::Native("uluna".to_string()),
      emissions_per_week: u(100),
      mint_config: msgs_connector_emission::MintConfig::MintDirect,
      rebase_config: msgs_connector_emission::RebaseConfg::Dynamic {},
      team_share: Decimal::percent(10),
    };

    self.addresses.ve3_connector_emissions =
      self.init_contract(code_id, msg, "ve3_connector_emissions");
  }

  pub(super) fn create_connector_alliance_1(&mut self) {
    let code_id = self.app.store_code(alliance_rewards_mock());

    let msg = alliance_rewards_mock::InstantiateMsg {
      reward_denom: "uluna".to_string(),
    };

    self.addresses.ve3_connector_alliance_mock =
      self.init_contract(code_id, msg, "alliance_rewards_mock_1");
  }

  // fn create_connector_alliance_2(&mut self) {
  //   let code_id = self.app.store_code(alliance_rewards_mock());

  //   let msg = alliance_rewards_mock::InstantiateMsg {
  //     reward_denom: "uluna".to_string(),
  //   };

  //   self.addresses.ve3_connector_alliance_2 =
  //     self.init_contract(code_id, msg, "alliance_rewards_mock_2");
  // }

  pub(super) fn create_voting_escrow(&mut self) {
    let code_id = self.app.store_code(ve3_voting_escrow());

    let msg = ve3_shared::msgs_voting_escrow::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      deposit_assets: vec![
        DepositAsset {
          info: AssetInfoUnchecked::native("uluna"),
          config: ve3_shared::msgs_voting_escrow::AssetInfoConfig::Default,
        },
        DepositAsset {
          info: self.addresses.ampluna_info(),
          config: ve3_shared::msgs_voting_escrow::AssetInfoConfig::ExchangeRate {
            contract: self.addresses.eris_hub.clone(),
          },
        },
      ],
    };

    self.addresses.ve3_voting_escrow = self.init_contract(code_id, msg, "ve3_voting_escrow");
  }

  pub(super) fn create_zapper_mock(&mut self) {
    let code_id = self.app.store_code(ve3_zapper_mock());

    let addr = self.addresses.clone();

    let msg = crate::mocks::zapper_mock::InstantiateMsg {
      exchange_rate: vec![(
        addr.uluna_info_checked(),
        addr.usdc_info_checked(),
        Decimal::percent(30),
      )],
      assets: vec![addr.uluna(100_000000), addr.usdc(100_000000)].into(),
    };

    self.addresses.ve3_zapper = self.init_contract(code_id, msg, "ve3_zapper");
    self
      .app
      .send_tokens(
        self.address("user1"),
        self.addresses.ve3_zapper.clone(),
        &[addr.uluna(100_000000).to_coin().unwrap(), addr.usdc(100_000000).to_coin().unwrap()],
      )
      .unwrap();
  }

  pub(super) fn create_pdt(&mut self) {
    let code_id = self.app.store_code(pdt());

    let msg = msgs_phoenix_treasury::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      alliance_token_denom: "vt".to_string(),
      reward_denom: "uluna".to_string(),
      oracles: vec![],
      vetos: vec![],
    };

    self.addresses.pdt = self.init_contract(code_id, msg, "phoenix_treasury");
  }

  pub(super) fn create_hub_cw20(&mut self) {
    let code_id = self.app.store_code(eris_hub_cw20_mock());
    self.addresses.eris_hub_cw20_code = code_id;
  }

  pub(super) fn create_incentive_mock(&mut self) {
    let code_id = self.app.store_code(incentive_mock());

    let astro = AssetInfoBase::Native("astro".to_string());
    let msg = incentive_mock::InstantiateMsg {
      config: incentive_mock::Config {
        emission: astro.clone(),
        per_week: u(10000),
        per_week_xxx: u(5000),
      },
    };

    self.addresses.incentive_mock = self.init_contract(code_id, msg, "incentive_mock");

    self
      .app
      .execute(
        self.address("creator"),
        astro
          .with_balance(Uint128::new(1_000_000_000_000u128))
          .transfer_msg(self.addresses.incentive_mock.to_string())
          .unwrap(),
      )
      .unwrap();
  }

  pub(super) fn create_astroport_pair_mock(&mut self) {
    let code_id = self.app.store_code(astroport_pair_mock());
    let msg = astroport_pair_mock::InstantiateMsg {
      price: Decimal::from_ratio(30u128, 100u128),
    };

    self.addresses.astroport_pair_mock = self.init_contract(code_id, msg, "incentive_mock");
  }

  pub(super) fn create_fake_cw20(&mut self) {
    let code_id = self.app.store_code(eris_hub_cw20_mock());

    let msg = cw20_base::msg::InstantiateMsg {
      decimals: 6,
      name: "fake".to_string(),
      symbol: "fake".to_string(),
      initial_balances: vec![
        Cw20Coin {
          address: self.creator().to_string(),
          amount: Uint128::new(100_000_000_000_000u128),
        },
        Cw20Coin {
          address: self.user1().to_string(),
          amount: Uint128::new(100_000_000_000_000u128),
        },
        Cw20Coin {
          address: self.user2().to_string(),
          amount: Uint128::new(100_000_000_000_000u128),
        },
      ],
      mint: Some(cw20::MinterResponse {
        minter: self.creator().to_string(),
        cap: None,
      }),
      marketing: None,
    };

    self.addresses.fake_cw20 = self.init_contract(code_id, msg, "fake_cw20");
  }

  pub(super) fn create_lp_cw20(&mut self) {
    let code_id = self.app.store_code(eris_hub_cw20_mock());

    let msg = cw20_base::msg::InstantiateMsg {
      decimals: 6,
      name: "lp_awesome".to_string(),
      symbol: "ulp".to_string(),
      initial_balances: vec![
        Cw20Coin {
          address: self.creator().to_string(),
          amount: Uint128::new(100_000_000_000_000u128),
        },
        Cw20Coin {
          address: self.user1().to_string(),
          amount: Uint128::new(100_000_000_000_000u128),
        },
        Cw20Coin {
          address: self.user2().to_string(),
          amount: Uint128::new(100_000_000_000_000u128),
        },
      ],
      mint: Some(cw20::MinterResponse {
        minter: self.creator().to_string(),
        cap: None,
      }),
      marketing: None,
    };

    self.addresses.lp_cw20 = self.init_contract(code_id, msg, "lp");
  }

  pub(super) fn create_hub_eris(&mut self) {
    let code_id = self.app.store_code(eris_hub());

    let msg = eris::hub::InstantiateMsg {
      cw20_code_id: self.addresses.eris_hub_cw20_code,
      owner: self.creator().to_string(),
      name: "aLUNA".to_string(),
      symbol: "aLUNA".to_string(),
      decimals: 6,
      epoch_period: 3 * 24 * 60 * 60,
      unbond_period: 21 * 24 * 60 * 60 + 14,
      validators: vec![self.address("val1").to_string(), self.address("val2").to_string()],
      protocol_fee_contract: self.address("fee").to_string(),
      protocol_reward_fee: Decimal::percent(10),
      delegation_strategy: Some(DelegationStrategy::Uniform),
      vote_operator: None,
    };

    self.addresses.eris_hub = self.init_contract(code_id, msg, "eris_hub");

    let config: eris::hub::ConfigResponse = self
      .app
      .wrap()
      .query_wasm_smart(self.addresses.eris_hub.clone(), &eris::hub::QueryMsg::Config {})
      .unwrap();

    self.addresses.eris_hub_cw20_ampluna = Addr(&config.stake_token);
  }
}
