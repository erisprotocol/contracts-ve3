use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::{Bound, Item, Map};
use ve3_shared::helpers::bps::BasicPoints;
use ve3_shared::msgs_asset_gauge::{Config, GaugeDistributionPeriod};
use ve3_shared::msgs_voting_escrow::LockInfoResponse;

use crate::period_index::PeriodIndex;

pub const CONFIG: Item<Config> = Item::new("config");
pub const LOCK_INFO: Map<&str, LockInfoResponse> = Map::new("lock_info");
pub const GAUGE_DISTRIBUTION: Map<(&str, u64), GaugeDistributionPeriod> =
  Map::new("gauge_distribution");

// gauge -> user -> period = votes
pub const GAUGE_VOTE: Map<(&str, &str, u64), UserVotes> = Map::new("gauge_vote");

pub fn fetch_last_gauge_vote(
  storage: &dyn Storage,
  gauge: &str,
  user: &str,
  period: u64,
) -> StdResult<Option<(u64, UserVotes)>> {
  let period_opt = GAUGE_VOTE
    .prefix((gauge, user))
    .range(storage, None, Some(Bound::inclusive(period)), Order::Descending)
    .next()
    .transpose()?;

  Ok(period_opt)
}

pub fn fetch_first_gauge_vote(
  storage: &dyn Storage,
  gauge: &str,
  user: &str,
) -> StdResult<Option<(u64, UserVotes)>> {
  let period_opt = GAUGE_VOTE
    .prefix((gauge, user))
    .range(storage, None, None, Order::Ascending)
    .next()
    .transpose()?;

  Ok(period_opt)
}

pub fn fetch_last_gauge_distribution(
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
  pub votes: Vec<(String, BasicPoints)>,
}

pub fn user_idx<'a>() -> PeriodIndex<'a> {
  PeriodIndex::new("user_info", "user_slope_changes", "user_keys")
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

  pub fn idx(&self) -> PeriodIndex<'_> {
    PeriodIndex::new(&self.data_key, &self.slope_key, &self.keys_key)
  }
}
