use super::helpers::{cw20, native, Addr};
use crate::mocks::stargate_mock::StargateMockModule;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{coin, Addr, BlockInfo, Coin, Decimal, Empty, Timestamp, Uint128, Validator};
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked};
use cw_multi_test::{
  App, AppBuilder, BankKeeper, DistributionKeeper, FailingModule, GovFailingModule,
  IbcFailingModule, MockAddressGenerator, MockApiBech32, StakeKeeper, StakingInfo, WasmKeeper,
};
use std::str::FromStr;
use std::vec;
use ve3_shared::constants::*;
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;

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

#[derive(Default)]
pub struct InitOptions {
  pub rebase_asset: Option<AssetInfoUnchecked>,
}

#[cw_serde]
pub struct Addresses {
  pub senders: [Addr; 3],

  pub creator: Addr,
  pub user1: Addr,
  pub user2: Addr,
  pub dca1: Addr,

  pub ve3_asset_gauge: Addr,
  pub ve3_bribe_manager: Addr,
  pub ve3_global_config: Addr,
  pub ve3_voting_escrow: Addr,
  pub ve3_zapper: Addr,
  pub pdt: Addr,

  pub ve3_asset_staking_1: Addr,
  pub ve3_connector_alliance_mock: Addr,

  pub ve3_asset_staking_2: Addr,
  pub ve3_connector_alliance_eris: Addr,

  pub ve3_asset_staking_3: Addr,
  pub ve3_connector_emissions: Addr,

  pub eris_hub: Addr,
  pub eris_hub_cw20_ampluna: Addr,
  // pub eris_hub_mock: Addr,
  // pub eris_hub_cw20_mock: Addr,
  pub eris_hub_cw20_code: u64,

  pub fake_cw20: Addr,
  pub lp_cw20: Addr,
  pub fee_recipient: Addr,

  pub incentive_mock: Addr,
  pub astroport_pair_mock: Addr,

  pub active_asset_staking: Addr,
  pub active_connector_alliance: Addr,

  pub zasset_denom: String,
  pub gauge_1: String,
  pub gauge_2: String,
  pub gauge_3: String,
}

#[allow(dead_code)]
impl Addresses {
  pub(crate) fn zasset_info(&self) -> AssetInfoUnchecked {
    AssetInfoUnchecked::native(self.zasset_denom.to_string())
  }
  pub(crate) fn zasset_info_checked(&self) -> AssetInfo {
    AssetInfo::native(self.zasset_denom.to_string())
  }
  pub(crate) fn zasset(&self, a: u32) -> Asset {
    self.zasset_info_checked().with_balance_u128(a.into())
  }

  pub(crate) fn ampluna_info(&self) -> AssetInfoUnchecked {
    AssetInfoUnchecked::cw20(self.eris_hub_cw20_ampluna.to_string())
  }
  pub(crate) fn ampluna_info_checked(&self) -> AssetInfo {
    AssetInfo::cw20(self.eris_hub_cw20_ampluna.clone())
  }
  pub(crate) fn ampluna(&self, a: u32) -> Asset {
    cw20(self.eris_hub_cw20_ampluna.clone(), Uint128::new(a.into()))
  }

  pub(crate) fn lp_cw20_info(&self) -> AssetInfoUnchecked {
    AssetInfoUnchecked::cw20(self.lp_cw20.to_string())
  }
  pub(crate) fn lp_cw20_info_checked(&self) -> AssetInfo {
    AssetInfo::cw20(self.lp_cw20.clone())
  }
  #[allow(dead_code)]
  pub(crate) fn lp_cw20(&self, a: u32) -> Asset {
    cw20(self.lp_cw20.clone(), Uint128::new(a.into()))
  }

  pub(crate) fn lp_native_info(&self) -> AssetInfoUnchecked {
    AssetInfoUnchecked::native("lp".to_string())
  }
  pub(crate) fn lp_native_info_checked(&self) -> AssetInfo {
    AssetInfo::native("lp".to_string())
  }
  pub(crate) fn lp_native(&self, a: u32) -> Asset {
    native("lp", Uint128::new(a.into()))
  }

  pub(crate) fn uluna_info(&self) -> AssetInfoUnchecked {
    AssetInfoUnchecked::native("uluna".to_string())
  }
  pub(crate) fn uluna_info_checked(&self) -> AssetInfo {
    AssetInfo::native("uluna".to_string())
  }
  pub(crate) fn uluna(&self, a: u128) -> Asset {
    native("uluna", Uint128::new(a))
  }

  pub(crate) fn usdc_info(&self) -> AssetInfoUnchecked {
    AssetInfoUnchecked::native("ibc/usdc".to_string())
  }
  pub(crate) fn usdc_info_checked(&self) -> AssetInfo {
    AssetInfo::native("ibc/usdc".to_string())
  }
  pub(crate) fn usdc(&self, a: u128) -> Asset {
    native("ibc/usdc", Uint128::new(a))
  }

  pub(crate) fn fake_cw20(&self, a: u32) -> Asset {
    cw20(self.fake_cw20.clone(), Uint128::new(a.into()))
  }
  pub(crate) fn fake_native(&self, a: u32) -> Asset {
    native("xxx", Uint128::new(a.into()))
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
  #[allow(dead_code)]
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

  pub(crate) fn add_seconds(&mut self, count: u64) -> &mut Self {
    let mut block_info = self.app.block_info();
    block_info.time = block_info.time.plus_seconds(count);
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
      coin(1_000_000_000_000_000_000u128, "uluna".to_string()),
      coin(1_000_000_000_000_000_000u128, "xxx".to_string()),
      coin(1_000_000_000_000_000_000u128, "usdc".to_string()),
      coin(1_000_000_000_000_000_000u128, "lp".to_string()),
      coin(1_000_000_000_000_000_000u128, "astro".to_string()),
      coin(1_000_000_000_000_000_000u128, "ibc/usdc".to_string()),
    ])
  }

  pub(crate) fn default_with_balances(initial_balance: Vec<Coin>) -> Self {
    let api = MockApiBech32::new("terra");

    let creator = api.addr_make("creator");
    let user1 = api.addr_make("user1");
    let user2 = api.addr_make("user2");
    let fee_recipient = api.addr_make("AT_FEE_COLLECTOR");
    let dca1 = api.addr_make("dca1");

    let bank = BankKeeper::new();

    let balances = vec![
      (creator.clone(), initial_balance.clone()),
      (user1.clone(), initial_balance.clone()),
      (user2.clone(), initial_balance.clone()),
    ];
    let staking = StakeKeeper::new();

    let block = BlockInfo {
      time: Timestamp::from_seconds(EPOCH_START),
      height: 0,
      chain_id: "".to_string(),
    };

    let validators = vec![
      api.addr_make("val1").to_string(),
      api.addr_make("val2").to_string(),
      api.addr_make("val3").to_string(),
      api.addr_make("val4").to_string(),
    ];

    let app = AppBuilder::new()
      // .with_api(MockApiBech32::new("terra"))
      .with_api(api)
      .with_wasm(WasmKeeper::default().with_address_generator(MockAddressGenerator))
      .with_bank(bank)
      .with_stargate(StargateMockModule {})
      .with_staking(staking)
      .build(|router, api, storage| {
        balances.into_iter().for_each(|(account, amount)| {
          router.bank.init_balance(storage, &account, amount).unwrap()
        });

        router
          .staking
          .setup(
            storage,
            StakingInfo {
              bonded_denom: "uluna".to_string(),
              unbonding_time: 60,
              apr: Decimal::percent(10),
            },
          )
          .unwrap();

        for val in validators {
          router
            .staking
            .add_validator(
              api,
              storage,
              &block,
              Validator {
                address: val,
                commission: Decimal::from_str("0.05").unwrap(),
                max_commission: Decimal::from_str("0.05").unwrap(),
                max_change_rate: Decimal::from_str("0.05").unwrap(),
              },
            )
            .unwrap();
        }
      });

    Self {
      app,

      addresses: Addresses {
        senders: [creator.clone(), user1.clone(), user2.clone()],

        ve3_asset_gauge: Addr(""),
        ve3_bribe_manager: Addr(""),
        ve3_global_config: Addr(""),
        ve3_voting_escrow: Addr(""),
        ve3_asset_staking_1: Addr(""),
        ve3_connector_alliance_mock: Addr(""),
        ve3_asset_staking_2: Addr(""),
        ve3_connector_alliance_eris: Addr(""),
        ve3_asset_staking_3: Addr(""),
        ve3_connector_emissions: Addr(""),
        pdt: Addr(""),

        eris_hub: Addr(""),
        eris_hub_cw20_ampluna: Addr(""),
        eris_hub_cw20_code: 0,
        ve3_zapper: Addr(""),
        // eris_hub_mock: Addr(""),
        // eris_hub_cw20_mock: Addr(""),
        fake_cw20: Addr(""),
        lp_cw20: Addr(""),
        creator,
        user1,
        user2,
        fee_recipient,
        dca1,

        active_asset_staking: Addr(""),
        active_connector_alliance: Addr(""),

        incentive_mock: Addr(""),
        astroport_pair_mock: Addr(""),

        zasset_denom: "".to_string(),
        gauge_1: "stable".to_string(),
        gauge_2: "project".to_string(),
        gauge_3: "emission".to_string(),
      },
    }
  }

  pub(crate) fn gauge1(&self) -> String {
    "stable".to_string()
  }

  pub(crate) fn gauge2(&self) -> String {
    "project".to_string()
  }

  pub(crate) fn gauge3(&self) -> String {
    "emission".to_string()
  }

  pub fn address(&self, address: &str) -> Addr {
    self.app.api().addr_make(address)
  }

  #[track_caller]

  pub(crate) fn init(&mut self) -> Addresses {
    self.init_no_config(InitOptions::default());

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

    self.addresses.clone()
  }

  pub(crate) fn init_options(&mut self, init: InitOptions) -> &mut Self {
    self.init_no_config(init);

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

  #[track_caller]
  pub(crate) fn init_no_config(&mut self, init: InitOptions) -> &mut Self {
    // April 4th 2024 15:00:00 UTC
    let timestamp = Timestamp::from_seconds(1712242800u64);

    self.set_time(timestamp);

    self.create_hub_cw20();
    self.create_hub_eris();
    self.create_fake_cw20();
    self.create_lp_cw20();

    self.create_global_config();
    self.create_asset_gauge(init);
    self.create_bribe_manager();
    self.create_connector_alliance_1();
    self.create_connector_alliance_eris();
    self.create_connector_emissions();
    self.create_voting_escrow();
    self.create_zapper_mock();
    self.create_pdt();

    self.create_asset_staking_1();
    self.create_asset_staking_2();
    self.create_asset_staking_3();

    self.use_connector_alliance_1();
    self.use_staking_1();

    self.create_incentive_mock();
    self.create_astroport_pair_mock();

    self.def_get_ampluna("creator", 100_000000);
    self.def_change_exchange_rate(Decimal::percent(120));

    self.def_get_ampluna("user1", 10_000000);
    self.def_get_ampluna("user2", 10_000000);

    self
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
        (AT_TEAM_WALLET.to_string(), self.address("AT_TEAM_WALLET").to_string()),
        // pdt
        (PDT_CONFIG_OWNER.to_string(), self.address("PDT_CONFIG_OWNER").to_string()),
        (PDT_CONTROLLER.to_string(), self.address("PDT_CONTROLLER").to_string()),
        // (PDT_VETO_CONFIG_OWNER.to_string(), self.address("PDT_VETO_CONFIG_OWNER").to_string()),
        // contracts
        (AT_VOTING_ESCROW.to_string(), self.addresses.ve3_voting_escrow.to_string()),
        (AT_ASSET_GAUGE.to_string(), self.addresses.ve3_asset_gauge.to_string()),
        (AT_BRIBE_MANAGER.to_string(), self.addresses.ve3_bribe_manager.to_string()),
        (AT_ZAPPER.to_string(), self.addresses.ve3_zapper.to_string()),
        (at_connector(&self.gauge1()), self.addresses.ve3_connector_alliance_mock.to_string()),
        (at_connector(&self.gauge2()), self.addresses.ve3_connector_alliance_eris.to_string()),
        (at_connector(&self.gauge3()), self.addresses.ve3_connector_emissions.to_string()),
        (at_asset_staking(&self.gauge1()), self.addresses.ve3_asset_staking_1.to_string()),
        (at_asset_staking(&self.gauge2()), self.addresses.ve3_asset_staking_2.to_string()),
        (at_asset_staking(&self.gauge3()), self.addresses.ve3_asset_staking_3.to_string()),
      ],
      vec![
        (
          AT_FREE_BRIBES.to_string(),
          vec![
            self.addresses.ve3_asset_staking_1.to_string(),
            self.addresses.ve3_asset_staking_2.to_string(),
            self.addresses.ve3_asset_staking_3.to_string(),
            self.addresses.creator.to_string(),
          ],
        ),
        (PDT_DCA_EXECUTOR.to_string(), vec![self.addresses.dca1.to_string()]),
      ],
      "creator",
      |a| {
        a.unwrap();
      },
    )
  }
}

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
