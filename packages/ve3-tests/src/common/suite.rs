use super::helpers::{cw20, native, Uint128};
use crate::common::suite_contracts::*;
use crate::mocks::stargate_mock::StargateMockModule;
use crate::mocks::{alliance_rewards_mock, eris_hub_mock};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, Timestamp, Uint128};
use cw20::Cw20Coin;
use cw_asset::{Asset, AssetInfo, AssetInfoBase, AssetInfoUnchecked, AssetUnchecked};
use cw_multi_test::{
  App, AppBuilder, BankKeeper, DistributionKeeper, Executor, FailingModule, GovFailingModule,
  IbcFailingModule, MockAddressGenerator, MockApiBech32, StakeKeeper, WasmKeeper,
};
use serde::Serialize;
use std::str::FromStr;
use ve3_shared::msgs_asset_gauge::GaugeConfig;
use ve3_shared::msgs_voting_escrow::DepositAsset;
use ve3_shared::{constants::*, msgs_connector_alliance, msgs_connector_emission};

type OsmosisTokenFactoryApp = App<
  BankKeeper,
  MockApiBech32,
  MockStorage,
  FailingModule<Empty, Empty, Empty>,
  WasmKeeper<Empty, Empty>,
  StakeKeeper,
  DistributionKeeper,
  IbcFailingModule,
  GovFailingModule,
  StargateMockModule,
>;

pub struct TestingSuite {
  pub app: OsmosisTokenFactoryApp,
  pub addresses: Addresses,
}

#[cw_serde]
pub struct Addresses {
  pub senders: [Addr; 3],

  pub creator: Addr,
  pub user1: Addr,
  pub user2: Addr,

  pub ve3_asset_gauge: Addr,
  pub ve3_bribe_manager: Addr,
  pub ve3_global_config: Addr,
  pub ve3_voting_escrow: Addr,

  pub ve3_asset_staking_1: Addr,
  pub ve3_connector_alliance_1: Addr,

  pub ve3_asset_staking_2: Addr,
  pub ve3_connector_alliance_2: Addr,

  pub eris_hub: Addr,
  pub eris_hub_cw20: Addr,

  pub fake_cw20: Addr,
  pub lp_cw20: Addr,
  pub fee_recipient: Addr,

  pub active_asset_staking: Addr,
  pub active_connector_alliance: Addr,

  pub gauge_1: String,
  pub gauge_2: String,
}

impl Addresses {
  pub(crate) fn ampluna_info(&self) -> AssetInfoUnchecked {
    AssetInfoUnchecked::cw20(self.eris_hub_cw20.to_string())
  }
  pub(crate) fn lp_cw_info(&self) -> AssetInfoUnchecked {
    AssetInfoUnchecked::cw20(self.lp_cw20.to_string())
  }
  pub(crate) fn lp_native_info(&self) -> AssetInfoUnchecked {
    AssetInfoUnchecked::native("lp".to_string())
  }

  pub(crate) fn ampluna_info_checked(&self) -> AssetInfo {
    AssetInfo::cw20(self.eris_hub_cw20.clone())
  }
  pub(crate) fn lp_cw20_info_checked(&self) -> AssetInfo {
    AssetInfo::cw20(self.lp_cw20.clone())
  }
  pub(crate) fn uluna_info_checked(&self) -> AssetInfo {
    AssetInfo::native("uluna".to_string())
  }
  pub(crate) fn lp_info_checked(&self) -> AssetInfo {
    AssetInfo::native("lp".to_string())
  }
  pub(crate) fn ampluna(&self, a: u32) -> Asset {
    cw20(self.eris_hub_cw20.clone(), Uint128::new(a.into()))
  }
  pub(crate) fn fake_cw20(&self, a: u32) -> Asset {
    cw20(self.fake_cw20.clone(), Uint128::new(a.into()))
  }
  pub(crate) fn lp_cw20(&self, a: u32) -> Asset {
    cw20(self.lp_cw20.clone(), Uint128::new(a.into()))
  }

  pub(crate) fn uluna(&self, a: u32) -> Asset {
    native("uluna", Uint128::new(a.into()))
  }
  pub(crate) fn fake_native(&self, a: u32) -> Asset {
    native("xxx", Uint128::new(a.into()))
  }
  pub(crate) fn lp_native(&self, a: u32) -> Asset {
    native("lp", Uint128::new(a.into()))
  }
}

/// TestingSuite helpers
#[cfg(test)]
impl TestingSuite {
  pub(crate) fn creator(&mut self) -> Addr {
    self.address("creator")
  }
  pub(crate) fn user1(&mut self) -> Addr {
    self.address("user1")
  }
  pub(crate) fn user2(&mut self) -> Addr {
    self.address("user2")
  }

  pub(crate) fn token1(&mut self) -> Addr {
    self.address("token")
  }

  pub(crate) fn set_time(&mut self, timestamp: Timestamp) -> &mut Self {
    let mut block_info = self.app.block_info();
    block_info.time = timestamp;
    self.app.set_block(block_info);

    self
  }

  pub(crate) fn add_one_period(&mut self) -> &mut Self {
    let mut block_info = self.app.block_info();
    block_info.time = block_info.time.plus_days(7);
    self.app.set_block(block_info);

    self
  }

  pub(crate) fn add_periods(&mut self, count: u64) -> &mut Self {
    let mut block_info = self.app.block_info();
    block_info.time = block_info.time.plus_days(7 * count);
    self.app.set_block(block_info);

    self
  }

  // pub(crate) fn add_one_epoch(&mut self) -> &mut Self {
  //   let creator = self.creator();

  //   self.add_one_day().create_epoch(creator, |res| {
  //     res.unwrap();
  //   });

  //   self
  // }
}

impl TestingSuite {
  pub fn def() -> Self {
    TestingSuite::default_with_balances(vec![
      coin(1_000_000_000_000u128, "uluna".to_string()),
      coin(1_000_000_000_000u128, "xxx".to_string()),
      coin(1_000_000_000_000u128, "usdc".to_string()),
      coin(1_000_000_000_000u128, "lp".to_string()),
    ])
  }

  pub(crate) fn default_with_balances(initial_balance: Vec<Coin>) -> Self {
    let api = MockApiBech32::new("terra");

    let creator = api.addr_make("creator");
    let user1 = api.addr_make("user1");
    let user2 = api.addr_make("user2");
    let fee_recipient = api.addr_make("AT_FEE_COLLECTOR");

    let bank = BankKeeper::new();

    let balances = vec![
      (creator.clone(), initial_balance.clone()),
      (user1.clone(), initial_balance.clone()),
      (user2.clone(), initial_balance.clone()),
    ];

    let app = AppBuilder::new()
      // .with_api(MockApiBech32::new("terra"))
      .with_api(api)
      .with_wasm(WasmKeeper::default().with_address_generator(MockAddressGenerator))
      .with_bank(bank)
      .with_stargate(StargateMockModule {})
      .build(|router, _api, storage| {
        balances.into_iter().for_each(|(account, amount)| {
          router.bank.init_balance(storage, &account, amount).unwrap()
        });
      });

    Self {
      app,

      addresses: Addresses {
        senders: [creator.clone(), user1.clone(), user2.clone()],

        ve3_asset_gauge: Addr::unchecked(""),
        ve3_bribe_manager: Addr::unchecked(""),
        ve3_global_config: Addr::unchecked(""),
        ve3_voting_escrow: Addr::unchecked(""),
        ve3_asset_staking_1: Addr::unchecked(""),
        ve3_connector_alliance_1: Addr::unchecked(""),
        ve3_asset_staking_2: Addr::unchecked(""),
        ve3_connector_alliance_2: Addr::unchecked(""),

        eris_hub: Addr::unchecked(""),
        eris_hub_cw20: Addr::unchecked(""),
        fake_cw20: Addr::unchecked(""),
        lp_cw20: Addr::unchecked(""),
        creator,
        user1,
        user2,
        fee_recipient,

        active_asset_staking: Addr::unchecked(""),
        active_connector_alliance: Addr::unchecked(""),

        gauge_1: "stable".to_string(),
        gauge_2: "project".to_string(),
      },
    }
  }

  pub(crate) fn gauge1(&self) -> String {
    "stable".to_string()
  }

  pub(crate) fn gauge2(&self) -> String {
    "project".to_string()
  }

  pub fn address(&self, address: &str) -> Addr {
    self.app.api().addr_make(address)
  }

  #[track_caller]
  pub(crate) fn init(&mut self) -> &mut Self {
    self.init_no_config();

    self.init_global_config();
    self.e_ve_update_config(
      None,
      Some(vec![self.addresses.ve3_asset_gauge.to_string()]),
      None,
      "creator",
      |res| {
        res.unwrap();
      },
    );

    self
  }

  pub fn migrate(&mut self) -> &mut Self {
    let addr = self.addresses.clone();

    let code_id = self.app.store_code(ve3_global_config());
    let msg = ve3_shared::msgs_global_config::MigrateMsg {};
    self.migrate_contract(&addr.ve3_global_config, code_id, msg);

    let code_id = self.app.store_code(ve3_asset_gauge());
    let msg = ve3_shared::msgs_asset_gauge::MigrateMsg {};
    self.migrate_contract(&addr.ve3_asset_gauge, code_id, msg);

    let code_id = self.app.store_code(ve3_asset_staking());
    let msg = ve3_shared::msgs_asset_staking::MigrateMsg {};
    self.migrate_contract(&addr.ve3_asset_staking_1, code_id, msg);
    let msg = ve3_shared::msgs_asset_staking::MigrateMsg {};
    self.migrate_contract(&addr.ve3_asset_staking_2, code_id, msg);

    let code_id = self.app.store_code(ve3_bribe_manager());
    let msg = ve3_shared::msgs_bribe_manager::MigrateMsg {};
    self.migrate_contract(&addr.ve3_bribe_manager, code_id, msg);

    let code_id = self.app.store_code(ve3_voting_escrow());
    let msg = ve3_shared::msgs_voting_escrow::MigrateMsg {};
    self.migrate_contract(&addr.ve3_voting_escrow, code_id, msg);

    // TEST ALLIANCE CONNECTOR
    let code_id = self.app.store_code(ve3_connector_alliance());
    let init = msgs_connector_alliance::InstantiateMsg {
      alliance_token_denom: "test".to_string(),
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      gauge: self.addresses.gauge_1.clone(),
      reward_denom: "uluna".to_string(),
    };
    let alliance_connector = self
      .app
      .instantiate_contract(
        code_id,
        addr.creator.clone(),
        &init,
        &[],
        "init-connector",
        Some(addr.creator.to_string()),
      )
      .unwrap();
    let code_id = self.app.store_code(ve3_connector_alliance());
    let msg = ve3_shared::msgs_connector_alliance::MigrateMsg {};
    self.migrate_contract(&alliance_connector, code_id, msg);

    // TEST EMISSION CONNECTOR
    let code_id = self.app.store_code(ve3_connector_emission());
    let init = msgs_connector_emission::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      gauge: self.addresses.gauge_1.clone(),
      emission_token: AssetInfoBase::Native("uluna".to_string()),
      emissions_per_week: Uint128(100),
      mint_config: msgs_connector_emission::MintConfig::MintDirect,
      rebase_config: msgs_connector_emission::RebaseConfg::Fixed(Decimal::percent(10)),
      team_share: Decimal::percent(10),
    };
    let emission_connector = self
      .app
      .instantiate_contract(
        code_id,
        addr.creator.clone(),
        &init,
        &[],
        "init-emission",
        Some(addr.creator.to_string()),
      )
      .unwrap();
    let code_id = self.app.store_code(ve3_connector_emission());
    let msg = ve3_shared::msgs_connector_emission::MigrateMsg {};
    self.migrate_contract(&emission_connector, code_id, msg);

    self
  }

  #[track_caller]
  pub(crate) fn init_no_config(&mut self) -> &mut Self {
    // April 4th 2024 15:00:00 UTC
    let timestamp = Timestamp::from_seconds(1712242800u64);

    self.set_time(timestamp);

    self.create_hub();
    self.create_hub_cw20();
    self.create_fake_cw20();
    self.create_lp_cw20();

    self.create_global_config();
    self.create_asset_gauge();
    self.create_asset_staking_1();
    self.create_asset_staking_2();
    self.create_bribe_manager();
    self.create_connector_alliance_1();
    self.create_connector_alliance_2();
    self.create_voting_escrow();

    self.use_connector_alliance_1();
    self.use_asset_staking_1();

    self
  }

  #[track_caller]
  fn init_contract<T: Serialize>(&mut self, code_id: u64, msg: T, name: &str) -> Addr {
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

  #[track_caller]
  fn migrate_contract<T: Serialize>(&mut self, contract: &Addr, code_id: u64, msg: T) {
    let creator = self.creator().clone();
    self.app.migrate_contract(creator, contract.clone(), &msg, code_id).unwrap();
  }

  fn create_global_config(&mut self) {
    let code_id = self.app.store_code(ve3_global_config());

    let msg = ve3_shared::msgs_global_config::InstantiateMsg {
      owner: self.creator().to_string(),
    };

    self.addresses.ve3_global_config = self.init_contract(code_id, msg, "ve3_global_config");
  }

  fn create_asset_gauge(&mut self) {
    let code_id = self.app.store_code(ve3_asset_gauge());

    let msg = ve3_shared::msgs_asset_gauge::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      rebase_asset: AssetInfoBase::Native("uluna".to_string()),
      gauges: vec![
        GaugeConfig {
          name: self.gauge1(),
          min_gauge_percentage: Decimal::percent(10),
        },
        GaugeConfig {
          name: self.gauge2(),
          min_gauge_percentage: Decimal::percent(0),
        },
      ],
    };

    self.addresses.ve3_asset_gauge = self.init_contract(code_id, msg, "ve3_asset_gauge");
  }

  fn create_asset_staking_1(&mut self) {
    let code_id = self.app.store_code(ve3_asset_staking());

    let msg = ve3_shared::msgs_asset_staking::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      reward_info: AssetInfoBase::Native("uluna".to_string()),
      default_yearly_take_rate: Decimal::percent(10),
      gauge: self.gauge1(),
    };

    self.addresses.ve3_asset_staking_1 = self.init_contract(code_id, msg, "ve3_asset_staking_1");
  }

  fn create_asset_staking_2(&mut self) {
    let code_id = self.app.store_code(ve3_asset_staking());

    let msg = ve3_shared::msgs_asset_staking::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      reward_info: AssetInfoBase::Native("uluna".to_string()),
      default_yearly_take_rate: Decimal::percent(10),
      gauge: self.gauge2(),
    };

    self.addresses.ve3_asset_staking_2 = self.init_contract(code_id, msg, "create_asset_staking_2");
  }

  fn create_bribe_manager(&mut self) {
    let code_id = self.app.store_code(ve3_bribe_manager());

    let ampluna = self.addresses.ampluna_info();
    let msg = ve3_shared::msgs_bribe_manager::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      whitelist: vec![
        AssetInfoUnchecked::native("uluna"),
        AssetInfoUnchecked::native("usdc"),
        AssetInfoUnchecked::cw20(self.token1()),
        ampluna,
      ],
      fee: AssetUnchecked::native("uluna", 10_000000u128),
    };

    self.addresses.ve3_bribe_manager = self.init_contract(code_id, msg, "ve3_bribe_manager");
  }

  fn create_connector_alliance_1(&mut self) {
    let code_id = self.app.store_code(alliance_rewards_mock());

    let msg = alliance_rewards_mock::InstantiateMsg {
      reward_denom: "uluna".to_string(),
    };

    self.addresses.ve3_connector_alliance_1 =
      self.init_contract(code_id, msg, "alliance_rewards_mock_1");
  }

  fn create_connector_alliance_2(&mut self) {
    let code_id = self.app.store_code(alliance_rewards_mock());

    let msg = alliance_rewards_mock::InstantiateMsg {
      reward_denom: "uluna".to_string(),
    };

    self.addresses.ve3_connector_alliance_2 =
      self.init_contract(code_id, msg, "alliance_rewards_mock_2");
  }

  fn create_voting_escrow(&mut self) {
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

  fn create_hub(&mut self) {
    let code_id = self.app.store_code(eris_hub_mock());

    let msg = eris_hub_mock::InstantiateMsg {
      exchange_rate: Decimal::from_str("1.2").unwrap(),
    };

    self.addresses.eris_hub = self.init_contract(code_id, msg, "eris_hub");
  }

  fn create_hub_cw20(&mut self) {
    let code_id = self.app.store_code(eris_hub_cw20_mock());

    let msg = cw20_base::msg::InstantiateMsg {
      decimals: 6,
      name: "ampLUNA".to_string(),
      symbol: "ampLUNA".to_string(),
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

    self.addresses.eris_hub_cw20 = self.init_contract(code_id, msg, "eris_hub_cw20");
  }

  fn create_fake_cw20(&mut self) {
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

  fn create_lp_cw20(&mut self) {
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

  fn init_global_config(&mut self) -> &mut TestingSuite {
    self.e_gc_set_addresses(
      vec![
        // controller
        (
          AT_DELEGATION_CONTROLLER.to_string(),
          self.address("AT_DELEGATION_CONTROLLER").to_string(),
        ),
        (
          AT_ASSET_WHITELIST_CONTROLLER.to_string(),
          self.address("AT_ASSET_WHITELIST_CONTROLLER").to_string(),
        ),
        (
          AT_BRIBE_WHITELIST_CONTROLLER.to_string(),
          self.address("AT_BRIBE_WHITELIST_CONTROLLER").to_string(),
        ),
        // (AT_GAUGE_CONTROLLER.to_string(), self.address("AT_GAUGE_CONTROLLER").to_string()),
        (AT_VE_GUARDIAN.to_string(), self.address("AT_VE_GUARDIAN").to_string()),
        // receivers
        (AT_TAKE_RECIPIENT.to_string(), self.address("AT_TAKE_RECIPIENT").to_string()),
        (AT_FEE_COLLECTOR.to_string(), self.address("AT_FEE_COLLECTOR").to_string()),
        // contracts
        (AT_VOTING_ESCROW.to_string(), self.addresses.ve3_voting_escrow.to_string()),
        (AT_ASSET_GAUGE.to_string(), self.addresses.ve3_asset_gauge.to_string()),
        (AT_BRIBE_MANAGER.to_string(), self.addresses.ve3_bribe_manager.to_string()),
        (at_connector(&self.gauge1()), self.addresses.ve3_connector_alliance_1.to_string()),
        (at_connector(&self.gauge2()), self.addresses.ve3_connector_alliance_2.to_string()),
        (at_asset_staking(&self.gauge1()), self.addresses.ve3_asset_staking_1.to_string()),
        (at_asset_staking(&self.gauge2()), self.addresses.ve3_asset_staking_2.to_string()),
      ],
      vec![(
        AT_FREE_BRIBES.to_string(),
        vec![
          self.addresses.ve3_asset_staking_1.to_string(),
          self.addresses.ve3_asset_staking_2.to_string(),
          self.addresses.creator.to_string(),
        ],
      )],
      "creator",
      |a| {
        a.unwrap();
      },
    )
  }
}

impl TestingSuite {}

impl TestingSuite {
  #[track_caller]
  pub fn print_block(&mut self, text: &str) -> &mut TestingSuite {
    println!("-------------------------------------------------");
    println!("-------------------------------------------------");
    println!("------ {text} -----------------------------------");
    println!("-------------------------------------------------");
    println!("-------------------------------------------------");

    self
  }
}
