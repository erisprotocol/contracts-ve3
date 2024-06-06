use cw_storage_plus::{Item, Map};
use ve3_shared::contract_bribe_manager::{BribeBucket, Config};

pub const CONFIG: Item<Config> = Item::new("config");
pub const BRIBE_BUCKETS: Map<u64, BribeBucket> = Map::new("bribe_buckets");
pub const BRIBE_CREATOR: Map<(&str, u64), BribeBucket> = Map::new("bribe_creator");
