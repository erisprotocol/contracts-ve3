use crate::{
  adapters::{
    asset_staking::AssetStaking, global_config_adapter::ConfigExt, voting_escrow::VotingEscrow,
  },
  constants::{at_asset_staking, AT_VOTING_ESCROW},
  error::SharedError,
  helpers::time::{Time, Times},
  msgs_asset_staking::AssetDistribution,
  msgs_voting_escrow::LockInfoResponse,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Deps, DepsMut, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_asset::{AssetInfo, AssetInfoUnchecked};

/// This structure describes the basic settings for creating a contract.
#[cw_serde]
pub struct InstantiateMsg {
  pub global_config_addr: String,
  pub gauges: Vec<GaugeConfig>,
  pub rebase_asset: AssetInfoUnchecked,
}

#[cw_serde]
pub struct GaugeConfig {
  pub name: String,
  pub min_gauge_percentage: Decimal,
}

/// This structure describes the execute messages available in the contract.
#[cw_serde]
pub enum ExecuteMsg {
  /// Vote allows a vAMP holder to cast votes on which validators should get the delegations
  Vote {
    gauge: String,
    votes: Vec<(String, u16)>,
  },

  /// Updates the vote for a specified user. Only can be called from the escrow_addr
  UpdateVote {
    token_id: String,
    lock_info: LockInfoResponse,
  },

  ClaimRebase {
    token_id: Option<String>,
  },

  AddRebase {},

  Receive(Cw20ReceiveMsg),

  /// Will snapshot the voting power for each asset and store it for bribe allocation, and sends the newest distribution to the asset staking to decide staking rewards.
  SetDistribution {},

  UpdateConfig {
    update_gauge: Option<GaugeConfig>,
    remove_gauge: Option<String>,
  },
}

#[cw_serde]
pub enum ReceiveMsg {
  AddRebase {},
}

/// This structure describes the query messages available in the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
  /// UserInfo returns information about a voter and the validators they voted for
  #[returns(UserInfoExtendedResponse)]
  UserInfo {
    user: String,
    time: Option<Time>,
  },
  #[returns(UserInfosResponse)]
  UserInfos {
    start_after: Option<String>,
    limit: Option<u32>,
    time: Option<Time>,
  },
  /// Config returns the contract configuration
  #[returns(Config)]
  Config {},

  /// This is one of the most important queries.
  /// Queries the user shares [per user; per gauge; per period; per lp] compared to the total vp of [per gauge; per period; per lp]
  /// Which is used to distribute voting bribes
  #[returns(UserSharesResponse)]
  UserShares {
    user: Addr,
    times: Option<Times>,
  },

  #[returns(UserFirstParticipationResponse)]
  UserFirstParticipation {
    user: Addr,
  },
  /// PoolInfo returns the latest voting power allocated to a specific pool (generator)
  #[returns(VotedInfoResponse)]
  GaugeInfo {
    gauge: String,
    key: String,
    time: Option<Time>,
  },

  /// ValidatorInfos returns the latest EMPs allocated to all active validators
  #[returns(GaugeInfosResponse)]
  GaugeInfos {
    gauge: String,
    keys: Option<Vec<String>>,
    time: Option<Time>,
  },

  #[returns(GaugeDistributionResponse)]
  Distribution {
    gauge: String,
    time: Option<Time>,
  },

  #[returns(Vec<GaugeDistributionResponse>)]
  Distributions {
    time: Option<Time>,
  },

  #[returns(Vec<GaugeDistributionResponse>)]
  LastDistributions {},

  #[returns(LastDistributionPeriodResponse)]
  LastDistributionPeriod {},

  #[returns(UserPendingRebaseResponse)]
  UserPendingRebase {
    user: Addr,
  },
}

#[cw_serde]
pub struct UserPendingRebaseResponse {
  pub rebase: Uint128,
}

#[cw_serde]
pub struct LastDistributionPeriodResponse {
  pub period: Option<u64>,
}

#[cw_serde]
pub struct UserFirstParticipationResponse {
  pub period: Option<u64>,
}

#[cw_serde]
pub struct UserSharesResponse {
  pub shares: Vec<UserShare>,
}

#[cw_serde]
pub struct UserShare {
  pub gauge: String,
  pub asset: AssetInfo,
  pub period: u64,

  pub user_vp: Uint128,
  pub total_vp: Uint128,
}

/// This structure describes a migration message.
/// We currently take no arguments for migrations.
#[cw_serde]
pub struct MigrateMsg {}

/// This structure describes the parameters returned when querying for the contract configuration.
#[cw_serde]
pub struct Config {
  pub global_config_addr: Addr,
  pub gauges: Vec<GaugeConfig>,
  pub rebase_asset: AssetInfo,
}

impl Config {
  pub fn assert_gauge(&self, gauge: &str) -> Result<&GaugeConfig, SharedError> {
    let gauge_config = self.gauges.iter().find(|a| a.name == gauge);
    gauge_config.ok_or_else(|| SharedError::NotFound(format!("gauge not found: {0}", gauge)))
  }

  // pub fn assert_gauge_controller(&self, deps: &DepsMut, sender: &Addr) -> Result<(), SharedError> {
  //   self.global_config().assert_has_access(&deps.querier, AT_GAUGE_CONTROLLER, sender)
  // }

  pub fn get_asset_staking(&self, deps: &Deps, gauge: &str) -> Result<AssetStaking, SharedError> {
    self.global_config().get_address(&deps.querier, &at_asset_staking(gauge)).map(AssetStaking)
  }

  pub fn get_voting_escrow(&self, deps: &DepsMut) -> Result<VotingEscrow, SharedError> {
    self.global_config().get_address(&deps.querier, AT_VOTING_ESCROW).map(VotingEscrow)
  }
}

/// This structure describes the response used to return voting information for a specific pool (generator).
#[cw_serde]
#[derive(Default)]
pub struct VotedInfoResponse {
  /// Dynamic voting power that voted for this validator
  pub voting_power: Uint128,
  /// fixed amount available
  pub fixed_amount: Uint128,
  /// The slope at which the amount of vAMP that voted for this validator will decay
  pub slope: Uint128,
}

/// The struct describes a response used to return a staker's vAMP lock position.
#[cw_serde]
#[derive(Default)]
pub struct UserInfoExtendedResponse {
  pub voting_power: Uint128,
  pub fixed_amount: Uint128,
  pub slope: Uint128,

  /// The vote distribution for all the validators the staker picked
  pub gauge_votes: Vec<GaugeVote>,
}

#[cw_serde]
#[derive(Default)]
pub struct GaugeVote {
  pub gauge: String,
  pub period: u64,
  pub votes: Vec<(String, u16)>,
}

#[cw_serde]
pub struct GaugeDistributionResponse {
  pub gauge: String,
  pub period: u64,
  pub total_gauge_vp: Uint128,
  pub assets: Vec<AssetDistribution>,
}

pub type UserInfosResponse = Vec<(Addr, VotedInfoResponse)>;

pub type GaugeInfosResponse = Vec<(String, VotedInfoResponse)>;
