use std::collections::HashMap;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Empty, Order, StdResult, Storage, Uint128};
use cw_asset::AssetInfo;
use cw_storage_plus::{Bound, Item, Map};
use ve3_shared::asset_gauge::{Config, UserInfoResponse};
use ve3_shared::contract_asset_staking::AssetDistribution;
use ve3_shared::helpers::bps::BasicPoints;
use ve3_shared::helpers::governance::{calc_voting_power, get_period};
use ve3_shared::voting_escrow::LockInfoResponse;

use crate::period_index::PeriodIndex;

/// This structure describes voting parameters for a specific validator.
#[cw_serde]
#[derive(Default)]
pub struct VotedInfo {
  pub voting_power: Uint128,
  pub slope: Uint128,
  pub fixed_amount: Uint128,
}

/// The struct describes last user's votes parameters.
#[cw_serde]
#[derive(Default)]
pub struct UserInfo {
  pub vote_ts: u64,
  pub voting_power: Uint128,
  pub slope: Uint128,
  pub lock_end: u64,
  pub votes: Vec<(String, BasicPoints)>,
  pub fixed_amount: Uint128,
}

impl UserInfo {
  /// The function converts [`UserInfo`] object into [`UserInfoResponse`].
  pub(crate) fn into_response(self, period: u64) -> StdResult<UserInfoResponse> {
    let votes = self
      .votes
      .into_iter()
      .map(|(validator_addr, bps)| (validator_addr, u16::from(bps)))
      .collect();

    let user_last_vote_period = get_period(self.vote_ts).unwrap_or(period);
    let vp_at_period =
      calc_voting_power(self.slope, self.voting_power, user_last_vote_period, period);

    Ok(UserInfoResponse {
      vote_ts: self.vote_ts,
      voting_power: self.voting_power,
      slope: self.slope,
      lock_end: self.lock_end,
      votes,
      fixed_amount: self.fixed_amount,
      current_power: self.fixed_amount.checked_add(vp_at_period)?,
    })
  }
}

/// Stores config at the given key.
pub const CONFIG: Item<Config> = Item::new("config");

/// Stores voting parameters per pool at a specific period by key ( period -> validator_addr ).
// pub const ASSET_VOTES: Map<(&str, u64), VotedInfo> = Map::new("asset_votes");
/// Hashset based on [`Map`]. It stores null object by key ( validator_addr -> period ).
/// This hashset contains all periods which have saved result in [`VALIDATOR_VOTES`] for a specific validator address.
// pub const ASSET_PERIODS: Map<(&str, u64), ()> = Map::new("asset_periods");

/// Slope changes for a specific validator address by key ( validator_addr -> period ).
// pub const ASSET_SLOPE_CHANGES: Map<(&str, u64), Uint128> = Map::new("asset_slope_changes");
// pub const ASSET_FIXED_VAMP: Map<(&str, u64), Uint128> = Map::new("asset_fixed_vamp");

// pub const USER_PERIODS: Map<(&Addr, u64), ()> = Map::new("user_periods");
// pub const USER_INFO: Map<(&Addr, u64), UserInfo> = Map::new("user_info");
// pub const USER_SLOPE_CHANGES: Map<(&Addr, u64), Uint128> = Map::new("user_slope_changes");

pub const LOCK_INFO: Map<&str, LockInfoResponse> = Map::new("lock_info");

pub const GAUGE_DISTRIBUTION: Map<(&str, u64), GaugeDistributionPeriod> =
  Map::new("gauge_distribution");

pub fn fetch_last_asset_distribution(
  storage: &dyn Storage,
  gauge: &str,
  period: u64,
) -> StdResult<Option<(u64, GaugeDistributionPeriod)>> {
  let period_opt = GAUGE_DISTRIBUTION
    .prefix(gauge)
    .range(storage, None, Some(Bound::inclusive(period)), Order::Descending)
    .next()
    .transpose()?;

  Ok(period_opt)
}

#[cw_serde]
#[derive(Default)]
pub struct UserVotes {
  pub period: u64,
  pub votes: Vec<(String, BasicPoints)>,
}

// type AssetIndex<'a> = PeriodIndex<'a, Empty>;
type UserIndex<'a> = PeriodIndex<'a, UserVotes>;

pub fn user_idx<'a>() -> UserIndex<'a> {
  UserIndex::new("user_info", "user_slope_changes", "user_keys")
}

pub struct AssetIndex {
  data_key: String,
  slope_key: String,
  keys_key: String,
}

impl AssetIndex {
  pub fn new(gauge: &str) -> Self {
    let data_key = format!("asset_votes__{0}", gauge);
    let slope_key = format!("asset_slope_changes__{0}", gauge);
    let keys_key = format!("asset_keys__{0}", gauge);
    Self {
      data_key,
      slope_key,
      keys_key,
    }
  }

  pub fn idx<'a>(&self) -> PeriodIndex<'a, Empty> {
    PeriodIndex::new(&self.data_key, &self.slope_key, &self.keys_key)
  }
}

// pub fn asset_idx<'a>(gauge: &str) -> AssetIndex<'a> {
//   AssetIndex::new(
//     &format!("asset_votes__{0}", gauge),
//     &format!("asset_slope_changes__{0}", gauge),
//     &format!("asset_keys__{0}", gauge),
//   )
// }

#[cw_serde]
pub struct GaugeDistributionPeriod {
  pub assets: Vec<AssetDistribution>,
}
