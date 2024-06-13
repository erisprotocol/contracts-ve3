use super::suite::TestingSuite;
use cosmwasm_std::{Addr, CosmosMsg, StdError, StdResult};
use cw20::Cw20ExecuteMsg;
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked, AssetUnchecked};
use cw_multi_test::{AppResponse, Executor};
use ve3_shared::{
  extensions::asset_ext::AssetExt,
  helpers::{assets::Assets, time::Time},
  msgs_bribe_manager::*,
};

#[allow(dead_code)]
impl TestingSuite {
  fn contract_bribe(&self) -> Addr {
    self.addresses.ve3_bribe_manager.clone()
  }

  #[track_caller]
  pub fn e_increase_allowance(
    &mut self,
    funds: Asset,
    spender: &str,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = funds.increase_allowance_msg(self.address(spender).to_string(), None).unwrap();
    let contract = if let AssetInfo::Cw20(addr) = funds.info {
      addr
    } else {
      panic!("{:?}", StdError::generic_err("not supported"))
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, contract, &msg, &[]));
    self
  }

  #[allow(clippy::too_many_arguments)]
  pub fn e_bribe_add_bribe_native(
    &mut self,
    funds: Asset,
    gauge: &str,
    for_info: AssetInfoUnchecked,
    distribution: BribeDistribution,
    fees: Option<Asset>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::AddBribe {
      bribe: funds.clone().into(),
      gauge: gauge.to_string(),
      for_info,
      distribution,
    };

    let mut combined = Assets::default();
    combined.add(&funds);
    if let Some(fees) = fees {
      combined.add(&fees);
    }

    match funds.info {
      cw_asset::AssetInfoBase::Native(_) => {
        let sender = self.address(sender);
        result(self.app.execute_contract(
          sender,
          self.contract_bribe(),
          &msg,
          &combined.get_coins().unwrap().into_vec(),
        ));
      },
      cw_asset::AssetInfoBase::Cw20(_) => todo!(),
      _ => todo!(),
    }

    self
  }

  #[allow(clippy::too_many_arguments)]
  pub fn e_bribe_add_bribe_cw20(
    &mut self,
    funds: Asset,
    gauge: &str,
    for_info: AssetInfoUnchecked,
    distribution: BribeDistribution,
    fees: Option<Asset>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::AddBribe {
      bribe: funds.clone().into(),
      gauge: gauge.to_string(),
      for_info,
      distribution,
    };

    let mut combined = Assets::default();
    if let Some(fees) = fees {
      combined.add(&fees);
    }

    match funds.info {
      cw_asset::AssetInfoBase::Native(_) => {},
      cw_asset::AssetInfoBase::Cw20(addr) => {
        let sender = self.address(sender);

        self
          .app
          .execute_contract(
            sender.clone(),
            addr,
            &Cw20ExecuteMsg::IncreaseAllowance {
              spender: self.contract_bribe().to_string(),
              amount: funds.amount,
              expires: Some(cw20::Expiration::AtHeight(self.app.block_info().height + 1)),
            },
            &[],
          )
          .unwrap();

        result(self.app.execute_contract(
          sender,
          self.contract_bribe(),
          &msg,
          &combined.get_coins().unwrap().into_vec(),
        ));
      },
      _ => todo!(),
    }

    self
  }

  pub fn e_bribe_withdraw_bribes(
    &mut self,
    period: u64,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::WithdrawBribes {
      period,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_bribe(), &msg, &[]));
    self
  }

  pub fn e_bribe_claim_bribes(
    &mut self,
    periods: Option<Vec<u64>>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ClaimBribes {
      periods,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_bribe(), &msg, &[]));
    self
  }

  pub fn e_bribe_whitelist_assets(
    &mut self,
    assets: Vec<AssetInfoUnchecked>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::WhitelistAssets(assets);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_bribe(), &msg, &[]));
    self
  }

  pub fn e_bribe_remove_assets(
    &mut self,
    assets: Vec<AssetInfoUnchecked>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::RemoveAssets(assets);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_bribe(), &msg, &[]));
    self
  }

  pub fn e_bribe_update_config(
    &mut self,
    fee: Option<AssetUnchecked>,
    allow_any: Option<bool>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateConfig {
      fee,
      allow_any,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_bribe(), &msg, &[]));
    self
  }

  pub fn q_bribe_config(&mut self, result: impl Fn(StdResult<Config>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract_bribe(), &QueryMsg::Config {});
    result(response);
    self
  }

  pub fn q_bribe_next_claim_period(
    &mut self,
    user: &str,
    result: impl Fn(StdResult<NextClaimPeriodResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_bribe(),
      &QueryMsg::NextClaimPeriod {
        user: self.address(user).to_string(),
      },
    );
    result(response);
    self
  }

  pub fn q_bribe_bribes(
    &mut self,
    period: Option<Time>,
    result: impl Fn(StdResult<BribesResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_bribe(),
      &QueryMsg::Bribes {
        period,
      },
    );
    result(response);
    self
  }

  pub fn q_bribe_user_claimable(
    &mut self,
    user: &str,
    periods: Option<Vec<u64>>,
    result: impl Fn(StdResult<UserClaimableResponse>),
  ) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(
      self.contract_bribe(),
      &QueryMsg::UserClaimable {
        user: self.address(user).to_string(),
        periods,
      },
    );
    result(response);
    self
  }
}
