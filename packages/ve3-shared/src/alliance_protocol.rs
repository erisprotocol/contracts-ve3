use crate::alliance_oracle_types::ChainId;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetInfo};
use std::collections::HashMap;

#[cw_serde]
pub struct Config {
    pub reward_denom: String,
    pub global_config_addr: Addr,
}

#[cw_serde]
pub struct AssetDistribution {
    pub asset: AssetInfo,
    pub distribution: Decimal,
}

#[cw_serde]
pub struct AssetConfig {
    pub yearly_take_rate: Decimal,
    pub last_taken_s: u64,
    pub stake_config: StakeConfig,
}

#[cw_serde]
pub enum StakeConfig {
    Default,
    Astroport {
        contract: String,
    },
    Ura {
        contract: String,
    },
}

#[cw_serde]
pub struct InstantiateMsg {
    pub global_config_addr: String,
    pub reward_denom: String,
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
    WhitelistAssets(HashMap<ChainId, Vec<AssetInfo>>),
    RemoveAssets(Vec<AssetInfo>),
    SetAssetRewardDistribution(Vec<AssetDistribution>),

    // operator
    UpdateRewards {},
    DistributeTakeRate {
        assets: Option<Vec<AssetInfo>>,
    },
    Callback(CallbackMsg),
}

#[cw_serde]
pub enum CallbackMsg {
    UpdateRewardsCallback {
        initial_balance: Asset,
    },
}

impl Into<ExecuteMsg> for CallbackMsg {
    fn into(self) -> ExecuteMsg {
        ExecuteMsg::Callback(self)
    }
}

#[cw_serde]
pub enum Cw20HookMsg {
    Stake {
        recipient: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},

    #[returns(WhitelistedAssetsResponse)]
    WhitelistedAssets {},

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

pub type WhitelistedAssetsResponse = HashMap<ChainId, Vec<AssetInfo>>;

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
}

#[cw_serde]
pub struct PendingRewardsRes {
    pub staked_asset: AssetInfo,
    pub reward_asset: AssetInfo,
    pub rewards: Uint128,
}
