use std::str::FromStr;

use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{Addr, Coin, Decimal, Empty, Timestamp, Uint128};
use cw20::Cw20Coin;
use cw_asset::{AssetInfoBase, AssetInfoUnchecked, AssetUnchecked};
use cw_multi_test::{
  App, AppBuilder, AppResponse, BankKeeper, DistributionKeeper, Executor, FailingModule,
  GovFailingModule, IbcFailingModule, MockAddressGenerator, MockApiBech32, StakeKeeper,
  StargateFailingModule, WasmKeeper,
};
use serde::Serialize;
use ve3_shared::constants::*;
use ve3_shared::msgs_asset_gauge::GaugeConfig;
use ve3_shared::msgs_global_config;
use ve3_shared::msgs_voting_escrow::DepositAsset;

use crate::common::suite_contracts::*;
use crate::mocks::{alliance_rewards_mock, eris_hub_mock};

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
  app: OsmosisTokenFactoryApp,
  pub senders: [Addr; 3],

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
}

/// TestingSuite helpers
#[cfg(test)]
impl TestingSuite {
  pub(crate) fn creator(&mut self) -> Addr {
    self.address("creator")
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

  // pub(crate) fn add_one_epoch(&mut self) -> &mut Self {
  //   let creator = self.creator();

  //   self.add_one_day().create_epoch(creator, |res| {
  //     res.unwrap();
  //   });

  //   self
  // }
}

impl TestingSuite {
  pub(crate) fn default_with_balances(initial_balance: Vec<Coin>) -> Self {
    let api = MockApiBech32::new("terra");

    let sender_1 = api.addr_make("creator");
    let sender_2 = api.addr_make("user1");
    let sender_3 = api.addr_make("user2");

    let bank = BankKeeper::new();

    let balances = vec![
      (sender_1.clone(), initial_balance.clone()),
      (sender_2.clone(), initial_balance.clone()),
      (sender_3.clone(), initial_balance.clone()),
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
      senders: [sender_1, sender_2, sender_3],

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
    }
  }

  pub(crate) fn ampluna(&self) -> AssetInfoUnchecked {
    AssetInfoUnchecked::cw20(self.eris_hub_cw20.to_string())
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
  pub(crate) fn instantiate_default(&mut self) -> &mut Self {
    // April 4th 2024 15:00:00 UTC
    let timestamp = Timestamp::from_seconds(1712242800u64);
    self.set_time(timestamp);

    self.create_hub();
    self.create_hub_cw20();

    self.create_global_config();
    self.create_asset_gauge();
    self.create_asset_staking_1();
    self.create_asset_staking_2();
    self.create_bribe_manager();
    self.create_connector_alliance_1();
    self.create_connector_alliance_2();
    self.create_voting_escrow();

    self.init_global_config();

    self
  }

  #[track_caller]
  fn init<T: Serialize>(&mut self, code_id: u64, msg: T, name: &str) -> Addr {
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

  fn create_global_config(&mut self) {
    let code_id = self.app.store_code(ve3_global_config());

    let msg = ve3_shared::msgs_global_config::InstantiateMsg {
      owner: self.creator().to_string(),
    };

    self.ve3_global_config = self.init(code_id, msg, "ve3_global_config");
  }

  fn create_asset_gauge(&mut self) {
    let code_id = self.app.store_code(ve3_asset_gauge());

    let msg = ve3_shared::msgs_asset_gauge::InstantiateMsg {
      global_config_addr: self.ve3_global_config.to_string(),
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

    self.ve3_asset_gauge = self.init(code_id, msg, "ve3_asset_gauge");
  }

  fn create_asset_staking_1(&mut self) {
    let code_id = self.app.store_code(ve3_asset_staking());

    let msg = ve3_shared::msgs_asset_staking::InstantiateMsg {
      global_config_addr: self.ve3_global_config.to_string(),
      reward_info: AssetInfoBase::Native("uluna".to_string()),
      default_yearly_take_rate: Decimal::percent(10),
      gauge: self.gauge1(),
    };

    self.ve3_asset_staking_1 = self.init(code_id, msg, "ve3_asset_staking_1");
  }

  fn create_asset_staking_2(&mut self) {
    let code_id = self.app.store_code(ve3_asset_staking());

    let msg = ve3_shared::msgs_asset_staking::InstantiateMsg {
      global_config_addr: self.ve3_global_config.to_string(),
      reward_info: AssetInfoBase::Native("uluna".to_string()),
      default_yearly_take_rate: Decimal::percent(10),
      gauge: self.gauge2(),
    };

    self.ve3_asset_staking_2 = self.init(code_id, msg, "create_asset_staking_2");
  }

  fn create_bribe_manager(&mut self) {
    let code_id = self.app.store_code(ve3_bribe_manager());

    let msg = ve3_shared::msgs_bribe_manager::InstantiateMsg {
      global_config_addr: self.ve3_global_config.to_string(),
      whitelist: vec![
        AssetInfoUnchecked::native("uluna"),
        AssetInfoUnchecked::native("usdc"),
        AssetInfoUnchecked::cw20(self.token1()),
      ],
      fee: AssetUnchecked::native("uluna", 10_000000u128),
    };

    self.ve3_bribe_manager = self.init(code_id, msg, "ve3_bribe_manager");
  }

  fn create_connector_alliance_1(&mut self) {
    let code_id = self.app.store_code(alliance_rewards_mock());

    let msg = alliance_rewards_mock::InstantiateMsg {
      reward_denom: "uluna".to_string(),
    };

    self.ve3_connector_alliance_1 = self.init(code_id, msg, "alliance_rewards_mock_1");
  }

  fn create_connector_alliance_2(&mut self) {
    let code_id = self.app.store_code(alliance_rewards_mock());

    let msg = alliance_rewards_mock::InstantiateMsg {
      reward_denom: "uluna".to_string(),
    };

    self.ve3_connector_alliance_2 = self.init(code_id, msg, "alliance_rewards_mock_2");
  }

  fn create_voting_escrow(&mut self) {
    let code_id = self.app.store_code(ve3_voting_escrow());

    let msg = ve3_shared::msgs_voting_escrow::InstantiateMsg {
      global_config_addr: self.ve3_global_config.to_string(),
      deposit_assets: vec![
        DepositAsset {
          info: AssetInfoUnchecked::native("uluna"),
          config: ve3_shared::msgs_voting_escrow::AssetInfoConfig::Default,
        },
        DepositAsset {
          info: self.ampluna(),
          config: ve3_shared::msgs_voting_escrow::AssetInfoConfig::ExchangeRate {
            contract: self.eris_hub.clone(),
          },
        },
      ],
    };

    self.ve3_voting_escrow = self.init(code_id, msg, "ve3_voting_escrow");
  }

  fn create_hub(&mut self) {
    let code_id = self.app.store_code(eris_hub_mock());

    let msg = eris_hub_mock::InstantiateMsg {
      exchange_rate: Decimal::from_str("1.2").unwrap(),
    };

    self.eris_hub = self.init(code_id, msg, "eris_hub");
  }

  fn create_hub_cw20(&mut self) {
    let code_id = self.app.store_code(eris_hub_cw20_mock());

    let msg = cw20_base::msg::InstantiateMsg {
      decimals: 6,
      name: "ampLUNA".to_string(),
      symbol: "ampLUNA".to_string(),
      initial_balances: vec![Cw20Coin {
        address: self.creator().to_string(),
        amount: Uint128::new(100_000_000_000_000u128),
      }],
      mint: Some(cw20::MinterResponse {
        minter: self.creator().to_string(),
        cap: None,
      }),
      marketing: None,
    };

    self.eris_hub_cw20 = self.init(code_id, msg, "eris_hub_cw20");
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
          (AT_VOTING_ESCROW.to_string(), self.ve3_voting_escrow.to_string()),
          (AT_ASSET_GAUGE.to_string(), self.ve3_asset_gauge.to_string()),
          (AT_BRIBE_MANAGER.to_string(), self.ve3_bribe_manager.to_string()),
          (at_connector(&self.gauge1()), self.ve3_connector_alliance_1.to_string()),
          (at_connector(&self.gauge2()), self.ve3_connector_alliance_2.to_string()),
          (at_asset_staking(&self.gauge1()), self.ve3_asset_staking_1.to_string()),
          (at_asset_staking(&self.gauge2()), self.ve3_asset_staking_2.to_string()),
        ],
        lists: vec![(
          AT_FREE_BRIBES.to_string(),
          vec![self.ve3_asset_staking_1.to_string(), self.ve3_asset_staking_2.to_string()],
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
    result(self.app.execute_contract(creator, self.ve3_global_config.clone(), &execute, &[]));
    self
  }

  #[track_caller]
  pub(crate) fn update_ownership(
    &mut self,
    sender: Addr,
    action: cw_ownable::Action,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut Self {
    let msg = msgs_global_config::ExecuteMsg::UpdateOwnership(action);
    result(self.app.execute_contract(sender, self.ve3_global_config.clone(), &msg, &[]));
    self
  }
}
