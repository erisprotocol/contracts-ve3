use cosmwasm_std::{Addr, Order, StdResult, Storage};
use cw_storage_plus::{Bound, Item, Map};
use ve3_shared::msgs_bribe_manager::{BribeBuckets, Config};

pub const CONFIG: Item<Config> = Item::new("config");
pub const BRIBE_AVAILABLE: Map<u64, BribeBuckets> = Map::new("bribe_available");
pub const BRIBE_TOTAL: Map<u64, BribeBuckets> = Map::new("bribe_totals");
pub const BRIBE_CREATOR: Map<(&str, u64), BribeBuckets> = Map::new("bribe_creator");
pub const BRIBE_CLAIMED: Map<(&str, u64), BribeBuckets> = Map::new("bribe_claimed");

pub fn fetch_last_claimed(
  storage: &dyn Storage,
  user: &str,
  period: u64,
) -> StdResult<Option<(u64, BribeBuckets)>> {
  let period_opt = BRIBE_CLAIMED
    .prefix(user)
    .range(storage, None, Some(Bound::inclusive(period)), Order::Descending)
    .next()
    .transpose()?;

  Ok(period_opt)
}

#[derive(Default)]
pub struct ClaimContext {
  pub period: u64,
  pub should_save: bool,
  pub skip: bool,

  pub bribe_available: BribeBuckets,
  pub bribe_totals: BribeBuckets,

  pub bribe_claimed: BribeBuckets,
}

impl ClaimContext {
  pub fn maybe_save(&self, store: &mut dyn Storage, user: &Addr) -> StdResult<()> {
    if self.period > 0 && self.should_save {
      BRIBE_CLAIMED.save(store, (user.as_str(), self.period), &self.bribe_claimed)?;
      BRIBE_AVAILABLE.save(store, self.period, &self.bribe_available)?;
    }

    Ok(())
  }
}
