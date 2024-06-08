use crate::{
  adapters::{bribe_manager::BribeManager, connector::Connector, global_config_adapter::ConfigExt},
  constants::{at_connector, AT_BRIBE_MANAGER},
  error::SharedError,
  stake_config::StakeConfig,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, DepsMut, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetInfo};

#[cw_serde]
pub struct Config {
  pub reward_info: AssetInfo,
  pub global_config_addr: Addr,
  pub default_yearly_take_rate: Decimal,
  pub gauge: String,
}

impl Config {
  pub fn get_connector(&self, deps: &DepsMut, gauge: &str) -> Result<Connector, SharedError> {
    Ok(Connector(self.get_address(&deps.querier, &at_connector(gauge))?))
  }
  pub fn get_bribe_manager(&self, deps: &DepsMut) -> Result<BribeManager, SharedError> {
    Ok(BribeManager(self.get_address(&deps.querier, AT_BRIBE_MANAGER)?))
  }
}

#[cw_serde]
pub struct AssetDistribution {
  pub asset: AssetInfo,
  pub distribution: Decimal,
  pub total_vp: Uint128,
}

#[cw_serde]
#[derive(Default)]
pub struct AssetConfigRuntime {
  pub last_taken_s: u64,
  pub taken: Uint128,
  pub harvested: Uint128,

  pub yearly_take_rate: Decimal,
  pub stake_config: StakeConfig,
}

#[cw_serde]
pub struct AssetConfig {
  pub yearly_take_rate: Decimal,
  pub stake_config: StakeConfig,
}

#[cw_serde]
pub struct InstantiateMsg {
  pub global_config_addr: String,
  pub reward_info: AssetInfo,
  pub default_yearly_take_rate: Decimal,
  pub gauge: String,
}

#[cw_serde]
pub struct AssetInfoWithConfig {
  pub info: AssetInfo,
  pub config: Option<AssetConfig>,
}

#[cw_serde]
pub struct AssetInfoWithRuntime {
  pub info: AssetInfo,
  pub config: AssetConfigRuntime,
  pub whitelisted: bool,
}

impl From<AssetInfo> for AssetInfoWithConfig {
  fn from(val: AssetInfo) -> Self {
    AssetInfoWithConfig::new(val, None)
  }
}

impl AssetInfoWithConfig {
  pub fn new(info: AssetInfo, config: Option<AssetConfig>) -> Self {
    Self {
      info,
      config,
    }
  }
}

#[cw_serde]
pub enum CallbackMsg {
  UpdateRewards {
    initial_balance: Asset,
  },
  TrackBribes {
    for_asset: AssetInfo,
    initial_balances: Vec<Asset>,
  },
  DistributeBribes {
    assets: Option<Vec<AssetInfo>>,
  },
}

impl From<CallbackMsg> for ExecuteMsg {
  fn from(val: CallbackMsg) -> Self {
    ExecuteMsg::Callback(val)
  }
}

#[cw_serde]
pub enum Cw20HookMsg {
  Stake {
    recipient: Option<String>,
  },
}

#[cw_serde]
pub enum ExecuteMsg {
  Receive(Cw20ReceiveMsg),

  // user
  Stake {
    recipient: Option<String>,
  },
  Unstake(Asset),
  ClaimRewards(AssetInfo),
  ClaimRewardsMultiple(Vec<AssetInfo>),

  // controller
  WhitelistAssets(Vec<AssetInfoWithConfig>),
  RemoveAssets(Vec<AssetInfo>),
  // cant update multiple as we need to track bribe recapturing
  UpdateAssetConfig(AssetInfoWithConfig),
  SetAssetRewardDistribution(Vec<AssetDistribution>),

  // operator
  UpdateRewards {},
  DistributeTakeRate {
    update: Option<bool>,
    assets: Option<Vec<AssetInfo>>,
  },
  DistributeBribes {
    update: Option<bool>,
    assets: Option<Vec<AssetInfo>>,
  },
  Callback(CallbackMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
  #[returns(Config)]
  Config {},

  #[returns(WhitelistedAssetsResponse)]
  WhitelistedAssets {},

  #[returns(WhitelistedAssetsDetailsResponse)]
  WhitelistedAssetDetails {},

  #[returns(Vec<AssetDistribution>)]
  RewardDistribution {},

  #[returns(StakedBalanceRes)]
  StakedBalance(AssetQuery),

  #[returns(PendingRewardsRes)]
  PendingRewards(AssetQuery),

  #[returns(Vec<StakedBalanceRes>)]
  AllStakedBalances(AllStakedBalancesQuery),

  #[returns(Vec<PendingRewardsRes>)]
  AllPendingRewards(AllPendingRewardsQuery),

  #[returns(Vec<StakedBalanceRes>)]
  TotalStakedBalances {},
}

pub type WhitelistedAssetsResponse = Vec<AssetInfo>;
pub type WhitelistedAssetsDetailsResponse = Vec<AssetInfoWithRuntime>;

#[cw_serde]
pub struct AssetQuery {
  pub address: String,
  pub asset: AssetInfo,
}

#[cw_serde]
pub struct AllStakedBalancesQuery {
  pub address: String,
}

#[cw_serde]
pub struct AllPendingRewardsQuery {
  pub address: String,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct StakedBalanceRes {
  pub asset: AssetInfo,
  pub balance: Uint128,
  pub shares: Uint128,
  pub config: AssetConfigRuntime,
}

#[cw_serde]
pub struct PendingRewardsRes {
  pub staked_asset: AssetInfo,
  pub reward_asset: AssetInfo,
  pub rewards: Uint128,
}
