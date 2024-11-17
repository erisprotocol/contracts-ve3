use super::suite::TestingSuite;
use cosmwasm_std::{to_json_binary, Addr, Coin, StdResult};
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetInfo};
use cw_multi_test::{AppResponse, Executor};
use ve3_shared::{extensions::asset_ext::AssetExt, msgs_asset_staking::*};

#[allow(dead_code)]
impl TestingSuite {
  fn contract_active_staking(&self) -> Addr {
    self.addresses.active_asset_staking.clone()
  }

  pub fn use_staking_1(&mut self) -> &mut TestingSuite {
    self.addresses.active_asset_staking = self.addresses.ve3_asset_staking_1.clone();
    self
  }

  pub fn use_staking_2(&mut self) -> &mut TestingSuite {
    self.addresses.active_asset_staking = self.addresses.ve3_asset_staking_2.clone();
    self
  }

  pub fn use_staking_3(&mut self) -> &mut TestingSuite {
    self.addresses.active_asset_staking = self.addresses.ve3_asset_staking_3.clone();
    self
  }

  pub fn e_staking_receive(
    &mut self,
    cw20_receive_msg: Cw20ReceiveMsg,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Receive(cw20_receive_msg);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[]));
    self
  }

  pub fn e_staking_stake(
    &mut self,
    recipient: Option<&str>,
    funds: Asset,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Stake {
      recipient: recipient.map(|a| self.address(a).to_string()),
    };

    match &funds.info {
      cw_asset::AssetInfoBase::Native(_) => {
        let coin: Coin = funds.to_coin().unwrap();
        let sender = self.address(sender);
        result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[coin]));
      },
      cw_asset::AssetInfoBase::Cw20(addr) => {
        let send_msg = cw20_base::msg::ExecuteMsg::Send {
          contract: self.contract_active_staking().to_string(),
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

  pub fn e_staking_unstake(
    &mut self,
    asset: Asset,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::Unstake {
      asset,
      recipient: None,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[]));
    self
  }

  pub fn e_staking_unstake_recipient(
    &mut self,
    asset: Asset,
    sender: &str,
    recipient: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let recipient = self.address(recipient);
    let msg = ExecuteMsg::Unstake {
      asset,
      recipient: Some(recipient.to_string()),
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[]));
    self
  }

  pub fn e_staking_claim_reward(
    &mut self,
    asset_info: AssetInfo,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ClaimReward {
      asset: asset_info,
      recipient: None,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[]));
    self
  }

  pub fn e_staking_claim_rewards(
    &mut self,
    asset_infos: Option<Vec<AssetInfo>>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::ClaimRewards {
      assets: asset_infos,
      recipient: None,
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[]));
    self
  }

  pub fn e_staking_whitelist_assets(
    &mut self,
    asset_infos: Vec<AssetInfoWithConfig<String>>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::WhitelistAssets(asset_infos);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[]));
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
    result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[]));
    self
  }

  pub fn e_staking_update_asset_config(
    &mut self,
    update_asset_config: AssetInfoWithConfig<String>,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateAssetConfig(update_asset_config);
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[]));
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
    result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[]));
    self
  }

  pub fn e_staking_update_rewards(
    &mut self,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let msg = ExecuteMsg::UpdateRewards {};
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[]));
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
    result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[]));
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
    result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[]));
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
    result(self.app.execute_contract(sender, self.contract_active_staking(), &msg, &[]));
    self
  }

  pub fn q_staking_config(&mut self, result: impl Fn(StdResult<Config>)) -> &mut Self {
    let response =
      self.app.wrap().query_wasm_smart(self.contract_active_staking(), &QueryMsg::Config {});
    result(response);
    self
  }

  pub fn q_staking_whitelisted_assets(
    &mut self,
    result: impl Fn(StdResult<WhitelistedAssetsResponse>),
  ) -> &mut Self {
    let response = self
      .app
      .wrap()
      .query_wasm_smart(self.contract_active_staking(), &QueryMsg::WhitelistedAssets {});
    result(response);
    self
  }

  pub fn q_staking_reward_distribution(
    &mut self,
    result: impl Fn(StdResult<Vec<AssetDistribution>>),
  ) -> &mut Self {
    let response = self
      .app
      .wrap()
      .query_wasm_smart(self.contract_active_staking(), &QueryMsg::RewardDistribution {});
    result(response);
    self
  }

  pub fn q_staking_staked_balance(
    &mut self,
    asset_query: AssetQuery,
    result: impl Fn(StdResult<StakedBalanceRes>),
  ) -> &mut Self {
    let response = self
      .app
      .wrap()
      .query_wasm_smart(self.contract_active_staking(), &QueryMsg::StakedBalance(asset_query));
    result(response);
    self
  }

  pub fn q_staking_pending_rewards(
    &mut self,
    asset_query: AssetQuery,
    result: impl Fn(StdResult<PendingRewardsRes>),
  ) -> &mut Self {
    let response = self
      .app
      .wrap()
      .query_wasm_smart(self.contract_active_staking(), &QueryMsg::PendingRewards(asset_query));
    result(response);
    self
  }

  pub fn q_staking_all_staked_balances(
    &mut self,
    query: AllStakedBalancesQuery,
    result: impl Fn(StdResult<Vec<StakedBalanceRes>>),
  ) -> &mut Self {
    let response = self
      .app
      .wrap()
      .query_wasm_smart(self.contract_active_staking(), &QueryMsg::AllStakedBalances(query));
    result(response);
    self
  }

  pub fn q_staking_all_pending_rewards(
    &mut self,
    query: AllPendingRewardsQuery,
    result: impl Fn(StdResult<Vec<PendingRewardsRes>>),
  ) -> &mut Self {
    let response = self
      .app
      .wrap()
      .query_wasm_smart(self.contract_active_staking(), &QueryMsg::AllPendingRewards(query));
    result(response);
    self
  }

  pub fn q_staking_all_pending_rewards_details(
    &mut self,
    query: AllPendingRewardsQuery,
    result: impl Fn(StdResult<Vec<PendingRewardsDetailRes>>),
  ) -> &mut Self {
    let response = self
      .app
      .wrap()
      .query_wasm_smart(self.contract_active_staking(), &QueryMsg::AllPendingRewardsDetail(query));
    result(response);
    self
  }

  pub fn q_staking_total_staked_balances(
    &mut self,
    result: impl Fn(StdResult<Vec<StakedBalanceRes>>),
  ) -> &mut Self {
    let response = self
      .app
      .wrap()
      .query_wasm_smart(self.contract_active_staking(), &QueryMsg::TotalStakedBalances {});
    result(response);
    self
  }

  pub fn q_staking_whitelisted_asset_details(
    &mut self,
    result: impl Fn(StdResult<WhitelistedAssetsDetailsResponse>),
  ) -> &mut Self {
    let response = self
      .app
      .wrap()
      .query_wasm_smart(self.contract_active_staking(), &QueryMsg::WhitelistedAssetDetails {});
    result(response);
    self
  }

  pub fn q_staking_pool_stakers(
    &mut self,
    query: PoolStakersQuery,
    result: impl Fn(StdResult<Vec<UserStakedBalanceRes>>),
  ) -> &mut Self {
    let response = self
      .app
      .wrap()
      .query_wasm_smart(self.contract_active_staking(), &QueryMsg::PoolStakers(query));
    result(response);
    self
  }
}
