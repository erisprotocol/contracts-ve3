use cosmwasm_std::{to_json_binary, Addr, Coin, Decimal, StdResult, Uint128};
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked, AssetUnchecked};
use cw_multi_test::{AppResponse, Executor};
use ve3_shared::{extensions::asset_ext::AssetExt, msgs_asset_compounding::*};

use super::suite::TestingSuite;

impl TestingSuite {
  fn contract_compounding(&self) -> Addr {
    self.addresses.ve3_asset_compounding.clone()
  }
  // // Receive method
  // pub fn e_compound_receive(
  //   &mut self,
  //   cw20_msg: Cw20ReceiveMsg,
  //   sender: &str,
  //   result: impl Fn(Result<AppResponse, anyhow::Error>),
  // ) -> &mut Self {
  //   let msg = ExecuteMsg::Receive(cw20_msg);
  //   let sender = self.address(sender);
  //   result(self.app.execute_contract(sender, self.contract_compounding(), &msg, &[]));
  //   self
  // }

  // Stake method
  pub fn e_compound_stake(
    &mut self,
    recipient: Option<&str>,
    gauge: &str,
    funds: Asset,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Stake {
      recipient: recipient.map(|a| self.address(a).to_string()),
      gauge: gauge.to_string(),
    };

    match &funds.info {
      cw_asset::AssetInfoBase::Native(_) => {
        let coin: Coin = funds.to_coin().unwrap();
        let sender = self.address(sender);
        result(self.app.execute_contract(sender, self.contract_compounding(), &msg, &[coin]));
      },
      cw_asset::AssetInfoBase::Cw20(addr) => {
        let send_msg = cw20_base::msg::ExecuteMsg::Send {
          contract: self.contract_compounding().to_string(),
          amount: funds.amount,
          msg: to_json_binary(&msg).unwrap(),
        };

        let sender = self.address(sender);
        result(self.app.execute_contract(sender, addr.clone(), &send_msg, &[]));
      },
      _ => panic!("not supported"),
    }

    self
  }

  // Unstake method
  pub fn e_compound_unstake(
    &mut self,
    recipient: Option<String>,
    sender: &str,
    funds: Vec<Coin>,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut Self {
    let msg = ExecuteMsg::Unstake {
      recipient,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_compounding(), &msg, &funds));
    self
  }

  // Compound method
  pub fn e_compound_compound(
    &mut self,
    minimum_receive: Option<Uint128>,
    asset_info: AssetInfoUnchecked,
    gauge: &str,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut Self {
    let msg = ExecuteMsg::Compound {
      minimum_receive,
      asset_info,
      gauge: gauge.to_string(),
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_compounding(), &msg, &[]));
    self
  }

  // InitializeAsset method
  pub fn e_compound_initialize_asset(
    &mut self,
    asset_info: AssetInfoUnchecked,
    gauge: &str,
    sender: &str,
    funds: Vec<Coin>,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut Self {
    let msg = ExecuteMsg::InitializeAsset {
      asset_info,
      gauge: gauge.to_string(),
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_compounding(), &msg, &funds));
    self
  }

  // UpdateConfig method
  #[allow(clippy::too_many_arguments)]
  pub fn e_compound_update_config(
    &mut self,
    fee: Option<Decimal>,
    fee_collector: Option<String>,
    deposit_profit_delay_s: Option<u64>,
    denom_creation_fee: Option<AssetUnchecked>,
    fee_for_assets: Option<Vec<(String, AssetInfoUnchecked, Option<Decimal>)>>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut Self {
    let msg = ExecuteMsg::UpdateConfig {
      fee,
      fee_collector,
      deposit_profit_delay_s,
      denom_creation_fee,
      fee_for_assets,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_compounding(), &msg, &[]));
    self
  }

  // Query for Config
  pub fn q_compound_config(&mut self, result: impl Fn(StdResult<Config>)) -> &mut Self {
    let response =
      self.app.wrap().query_wasm_smart(self.contract_compounding(), &QueryMsg::Config {});
    result(response);
    self
  }

  // Query for AssetConfig
  pub fn q_compound_asset_config(
    &mut self,
    asset_info: AssetInfo,
    gauge: &str,
    result: impl Fn(StdResult<CompoundingAssetConfig>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_compounding(),
      &QueryMsg::AssetConfig {
        asset_info,
        gauge: gauge.to_string(),
      },
    );
    result(response);
    self
  }

  pub fn q_compound_asset_configs(
    &mut self,
    assets: Option<Vec<(String, AssetInfo)>>,
    result: impl Fn(StdResult<Vec<CompoundingAssetConfig>>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_compounding(),
      &QueryMsg::AssetConfigs {
        assets,
      },
    );
    result(response);
    self
  }

  // Query for UserInfos
  pub fn q_compound_user_infos(
    &mut self,
    assets: Option<Vec<(String, AssetInfo)>>,
    addr: &str,
    result: impl Fn(StdResult<Vec<UserInfoResponse>>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_compounding(),
      &QueryMsg::UserInfos {
        assets,
        addr: self.address(addr).to_string(),
      },
    );
    result(response);
    self
  }

  // Query for ExchangeRates
  pub fn q_compound_exchange_rates(
    &mut self,
    assets: Option<Vec<(String, AssetInfo)>>,
    start_after: Option<u64>,
    limit: Option<u32>,
    result: impl Fn(StdResult<Vec<ExchangeRatesResponse>>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_compounding(),
      &QueryMsg::ExchangeRates {
        assets,
        start_after,
        limit,
      },
    );
    result(response);
    self
  }
}
