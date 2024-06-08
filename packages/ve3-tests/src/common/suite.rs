use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, Timestamp, Uint128};
use cw20::Cw20Coin;
use cw_asset::{Asset, AssetInfoBase, AssetInfoUnchecked, AssetUnchecked};
use cw_multi_test::{
  App, AppBuilder, AppResponse, BankKeeper, DistributionKeeper, Executor, FailingModule,
  GovFailingModule, IbcFailingModule, MockAddressGenerator, MockApiBech32, StakeKeeper,
  StargateFailingModule, WasmKeeper,
};
use serde::Serialize;
use ve3_shared::msgs_asset_gauge::GaugeConfig;
use ve3_shared::msgs_voting_escrow::DepositAsset;
use ve3_shared::{constants::*, msgs_asset_staking, msgs_bribe_manager};
use ve3_shared::{msgs_connector_alliance, msgs_global_config};

use crate::common::suite_contracts::*;
use crate::mocks::{alliance_rewards_mock, eris_hub_mock};

use super::helpers::cw20;

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
  StargateFailingModule,
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
}

impl Addresses {
  pub(crate) fn ampluna_info(&self) -> AssetInfoUnchecked {
    AssetInfoUnchecked::cw20(self.eris_hub_cw20.to_string())
  }

  pub(crate) fn ampluna<B: Into<Uint128>>(&self, a: B) -> Asset {
    cw20(self.eris_hub_cw20.clone(), a)
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
      coin(1_000_000_000u128, "uluna".to_string()),
      coin(1_000_000_000u128, "xxx".to_string()),
      coin(1_000_000_000u128, "usdc".to_string()),
    ])
  }

  pub(crate) fn default_with_balances(initial_balance: Vec<Coin>) -> Self {
    let api = MockApiBech32::new("terra");

    let creator = api.addr_make("creator");
    let user1 = api.addr_make("user1");
    let user2 = api.addr_make("user2");

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
      // .with_stargate(StargateMock {})
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
        creator,
        user1,
        user2,
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
    self.ve_update_config_execute(
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

    // cannot test for alliance connector
    // let code_id = self.app.store_code(ve3_connector_alliance());
    // let msg = ve3_shared::msgs_connector_alliance::MigrateMsg {};
    // self.migrate_contract(&addr.ve3_connector_alliance_1, code_id, msg);
    // let msg = ve3_shared::msgs_connector_alliance::MigrateMsg {};
    // self.migrate_contract(&addr.ve3_connector_alliance_2, code_id, msg);

    let code_id = self.app.store_code(ve3_voting_escrow());
    let msg = ve3_shared::msgs_voting_escrow::MigrateMsg {};
    self.migrate_contract(&addr.ve3_voting_escrow, code_id, msg);

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

    self.create_global_config();
    self.create_asset_gauge();
    self.create_asset_staking_1();
    self.create_asset_staking_2();
    self.create_bribe_manager();
    self.create_connector_alliance_1();
    self.create_connector_alliance_2();
    self.create_voting_escrow();

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
      gauges: vec![
        GaugeConfig {
          name: self.gauge1(),
          min_gauge_percentage: Decimal::percent(1),
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

    let msg = ve3_shared::msgs_bribe_manager::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      whitelist: vec![
        AssetInfoUnchecked::native("uluna"),
        AssetInfoUnchecked::native("usdc"),
        AssetInfoUnchecked::cw20(self.token1()),
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

  fn init_global_config(&mut self) -> &mut TestingSuite {
    self.global_config_execute(
      msgs_global_config::ExecuteMsg::SetAddresses {
        addresses: vec![
          // controller
          (
            AT_DELEGATION_CONTROLLER.to_string(),
            self.address("AT_DELEGATION_CONTROLLER").to_string(),
          ),
          (
            AT_ASSET_WHITELIST_CONTROLLER.to_string(),
            self.address("AT_ASSET_WHITELIST_CONTROLLER").to_string(),
          ),
          (AT_GAUGE_CONTROLLER.to_string(), self.address("AT_GAUGE_CONTROLLER").to_string()),
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
        lists: vec![(
          AT_FREE_BRIBES.to_string(),
          vec![
            self.addresses.ve3_asset_staking_1.to_string(),
            self.addresses.ve3_asset_staking_2.to_string(),
          ],
        )],
      },
      |a| {
        a.unwrap();
      },
    )
  }
}

impl TestingSuite {
  #[track_caller]
  pub fn global_config_execute(
    &mut self,
    execute: msgs_global_config::ExecuteMsg,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let creator = self.creator().clone();
    result(self.app.execute_contract(
      creator,
      self.addresses.ve3_global_config.clone(),
      &execute,
      &[],
    ));
    self
  }
}

impl TestingSuite {
  #[track_caller]
  pub fn bribe_execute(
    &mut self,
    execute: msgs_bribe_manager::ExecuteMsg,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let creator = self.creator().clone();
    result(self.app.execute_contract(
      creator,
      self.addresses.ve3_bribe_manager.clone(),
      &execute,
      &[],
    ));
    self
  }
}
impl TestingSuite {
  #[track_caller]
  pub fn connector_1_execute(
    &mut self,
    execute: msgs_connector_alliance::ExecuteMsg,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let creator = self.creator().clone();
    result(self.app.execute_contract(
      creator,
      self.addresses.ve3_connector_alliance_1.clone(),
      &execute,
      &[],
    ));
    self
  }

  #[track_caller]
  pub fn connector_2_execute(
    &mut self,
    execute: msgs_connector_alliance::ExecuteMsg,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let creator = self.creator().clone();
    result(self.app.execute_contract(
      creator,
      self.addresses.ve3_connector_alliance_2.clone(),
      &execute,
      &[],
    ));
    self
  }
}
impl TestingSuite {
  #[track_caller]
  pub fn staking_1_execute(
    &mut self,
    execute: msgs_asset_staking::ExecuteMsg,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let creator = self.creator().clone();
    result(self.app.execute_contract(
      creator,
      self.addresses.ve3_asset_staking_1.clone(),
      &execute,
      &[],
    ));
    self
  }

  #[track_caller]
  pub fn staking_2_execute(
    &mut self,
    execute: msgs_asset_staking::ExecuteMsg,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let creator = self.creator().clone();
    result(self.app.execute_contract(
      creator,
      self.addresses.ve3_asset_staking_2.clone(),
      &execute,
      &[],
    ));
    self
  }
}
