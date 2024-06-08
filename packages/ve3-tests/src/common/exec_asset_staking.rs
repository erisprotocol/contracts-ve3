use super::suite::TestingSuite;
use cosmwasm_std::{Addr, StdResult};
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetInfo};
use cw_multi_test::{AppResponse, Executor};
use ve3_shared::msgs_asset_staking::*;

#[allow(dead_code)]
impl TestingSuite {
  fn contract_4(&self) -> Addr {
    self.addresses.ve3_asset_staking_1.clone()
  }

  pub fn e_staking_receive(
    &mut self,
    cw20_receive_msg: Cw20ReceiveMsg,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Receive(cw20_receive_msg);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_4(), &msg, &[]));
    self
  }

  pub fn e_staking_stake(
    &mut self,
    recipient: Option<String>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Stake {
      recipient,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_4(), &msg, &[]));
    self
  }

  pub fn e_staking_unstake(
    &mut self,
    asset: Asset,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Unstake(asset);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_4(), &msg, &[]));
    self
  }

  pub fn e_staking_claim_rewards(
    &mut self,
    asset_info: AssetInfo,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ClaimRewards(asset_info);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_4(), &msg, &[]));
    self
  }

  pub fn e_staking_claim_rewards_multiple(
    &mut self,
    asset_infos: Vec<AssetInfo>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ClaimRewardsMultiple(asset_infos);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_4(), &msg, &[]));
    self
  }

  pub fn e_staking_whitelist_assets(
    &mut self,
    asset_infos: Vec<AssetInfo>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::WhitelistAssets(asset_infos);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_4(), &msg, &[]));
    self
  }

  pub fn e_staking_remove_assets(
    &mut self,
    asset_infos: Vec<AssetInfo>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::RemoveAssets(asset_infos);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_4(), &msg, &[]));
    self
  }

  pub fn e_staking_update_asset_config(
    &mut self,
    update_asset_config: UpdateAssetConfig,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateAssetConfig(update_asset_config);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_4(), &msg, &[]));
    self
  }

  pub fn e_staking_set_asset_reward_distribution(
    &mut self,
    asset_distributions: Vec<AssetDistribution>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::SetAssetRewardDistribution(asset_distributions);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_4(), &msg, &[]));
    self
  }

  pub fn e_staking_update_rewards(
    &mut self,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateRewards {};
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_4(), &msg, &[]));
    self
  }

  pub fn e_staking_distribute_take_rate(
    &mut self,
    update: Option<bool>,
    assets: Option<Vec<AssetInfo>>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::DistributeTakeRate {
      update,
      assets,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_4(), &msg, &[]));
    self
  }

  pub fn e_staking_distribute_bribes(
    &mut self,
    update: Option<bool>,
    assets: Option<Vec<AssetInfo>>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::DistributeBribes {
      update,
      assets,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_4(), &msg, &[]));
    self
  }

  pub fn e_staking_callback(
    &mut self,
    callback_msg: CallbackMsg,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Callback(callback_msg);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_4(), &msg, &[]));
    self
  }

  pub fn q_staking_config(&mut self, result: impl Fn(StdResult<Config>)) -> &mut Self {
    let response = self.app.wrap().query_wasm_smart(self.contract_4(), &QueryMsg::Config {});
    result(response);
    self
  }

  pub fn q_staking_whitelisted_assets(
    &mut self,
    result: impl Fn(StdResult<WhitelistedAssetsResponse>),
  ) -> &mut Self {
    let response =
      self.app.wrap().query_wasm_smart(self.contract_4(), &QueryMsg::WhitelistedAssets {});
    result(response);
    self
  }

  pub fn q_staking_reward_distribution(
    &mut self,
    result: impl Fn(StdResult<Vec<AssetDistribution>>),
  ) -> &mut Self {
    let response =
      self.app.wrap().query_wasm_smart(self.contract_4(), &QueryMsg::RewardDistribution {});
    result(response);
    self
  }

  pub fn q_staking_staked_balance(
    &mut self,
    asset_query: AssetQuery,
    result: impl Fn(StdResult<StakedBalanceRes>),
  ) -> &mut Self {
    let response =
      self.app.wrap().query_wasm_smart(self.contract_4(), &QueryMsg::StakedBalance(asset_query));
    result(response);
    self
  }

  pub fn q_staking_pending_rewards(
    &mut self,
    asset_query: AssetQuery,
    result: impl Fn(StdResult<PendingRewardsRes>),
  ) -> &mut Self {
    let response =
      self.app.wrap().query_wasm_smart(self.contract_4(), &QueryMsg::PendingRewards(asset_query));
    result(response);
    self
  }

  pub fn q_staking_all_staked_balances(
    &mut self,
    query: AllStakedBalancesQuery,
    result: impl Fn(StdResult<Vec<StakedBalanceRes>>),
  ) -> &mut Self {
    let response =
      self.app.wrap().query_wasm_smart(self.contract_4(), &QueryMsg::AllStakedBalances(query));
    result(response);
    self
  }

  pub fn q_staking_all_pending_rewards(
    &mut self,
    query: AllPendingRewardsQuery,
    result: impl Fn(StdResult<Vec<PendingRewardsRes>>),
  ) -> &mut Self {
    let response =
      self.app.wrap().query_wasm_smart(self.contract_4(), &QueryMsg::AllPendingRewards(query));
    result(response);
    self
  }

  pub fn q_staking_total_staked_balances(
    &mut self,
    result: impl Fn(StdResult<Vec<StakedBalanceRes>>),
  ) -> &mut Self {
    let response =
      self.app.wrap().query_wasm_smart(self.contract_4(), &QueryMsg::TotalStakedBalances {});
    result(response);
    self
  }
}
