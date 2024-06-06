use crate::{error::SharedError, voting_escrow::LockInfoResponse};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

/// This structure describes the basic settings for creating a contract.
#[cw_serde]
pub struct InstantiateMsg {
  pub global_config_addr: String,
  pub gauges: Vec<GaugeConfig>,
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

  /// TunePools transforms the latest vote distribution into alloc_points which are then applied to ASTRO generators
  SetDistribution {},

  ClearGaugeState {
    gauge: String,
    limit: Option<usize>,
  },

  UpdateConfig {
    update_gauge: Option<GaugeConfig>,
    remove_gauge: Option<String>,
  },
}

#[cw_serde]
pub struct GaugeConfig {
  pub name: String,
  pub min_gauge_percentage: Decimal,
  pub target: Addr,
}

/// This structure describes the query messages available in the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
  /// UserInfo returns information about a voter and the validators they voted for
  #[returns(UserInfoResponse)]
  UserInfo {
    user: String,
  },
  #[returns(UserInfosResponse)]
  UserInfos {
    start_after: Option<String>,
    limit: Option<u32>,
  },

  /// TuneInfo returns information about the latest generators that were voted to receive ASTRO emissions
  // #[returns(GaugeInfoResponse)]
  // TuneInfo {},

  /// Config returns the contract configuration
  #[returns(Config)]
  Config {},
  /// PoolInfo returns the latest voting power allocated to a specific pool (generator)
  #[returns(VotedValidatorInfoResponse)]
  ValidatorInfo {
    validator_addr: String,
  },
  /// PoolInfo returns the voting power allocated to a specific pool (generator) at a specific period
  #[returns(VotedValidatorInfoResponse)]
  ValidatorInfoAtPeriod {
    validator_addr: String,
    period: u64,
  },
  /// ValidatorInfos returns the latest EMPs allocated to all active validators
  #[returns(Vec<(String,VotedValidatorInfoResponse)>)]
  ValidatorInfos {
    validator_addrs: Option<Vec<String>>,
    period: Option<u64>,
  },
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
}

impl Config {
  pub fn assert_gauge(&self, name: &str) -> Result<&GaugeConfig, SharedError> {
    let gauge = self.gauges.iter().find(|a| a.name == name);
    gauge.ok_or_else(|| SharedError::NotFound(format!("gauge not found: {0}", name)))
  }
}

/// This structure describes the response used to return voting information for a specific pool (generator).
#[cw_serde]
#[derive(Default)]
pub struct VotedValidatorInfoResponse {
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
pub struct UserInfoResponse {
  /// Last timestamp when the user voted
  pub vote_ts: u64,
  /// The user's decreasing voting power
  pub voting_power: Uint128,
  /// The slope at which the user's voting power decays
  pub slope: Uint128,
  /// Timestamp when the user's lock expires
  pub lock_end: u64,
  /// The vote distribution for all the validators the staker picked
  pub votes: Vec<(String, u16)>,
  /// fixed amount available
  pub fixed_amount: Uint128,
  /// Current voting power at the current
  pub current_power: Uint128,
}

#[cw_serde]
#[derive(Default)]
pub struct UserInfosResponse {
  pub users: Vec<(Addr, UserInfoResponse)>,
}

// /// Queries amp tune info.
// pub fn get_amp_tune_info(
//     querier: &QuerierWrapper,
//     amp_gauge_addr: impl Into<String>,
// ) -> StdResult<GaugeInfoResponse> {
//     let gauge: GaugeInfoResponse =
//         querier.query_wasm_smart(amp_gauge_addr, &QueryMsg::TuneInfo {})?;
//     Ok(gauge)
// }

// pub fn get_amp_validator_infos(
//     querier: &QuerierWrapper,
//     amp_gauge_addr: impl Into<String>,
//     period: u64,
// ) -> StdResult<Vec<(String, VotedValidatorInfoResponse)>> {
//     let gauge: Vec<(String, VotedValidatorInfoResponse)> = querier.query_wasm_smart(
//         amp_gauge_addr,
//         &QueryMsg::ValidatorInfos {
//             validator_addrs: None,
//             period: Some(period),
//         },
//     )?;
//     Ok(gauge)
// }
