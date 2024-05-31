use cw_storage_plus::Item;
use std::collections::HashSet;
use ve3_shared::connector_alliance::Config;

pub const CONFIG: Item<Config> = Item::new("config");
pub const VALIDATORS: Item<HashSet<String>> = Item::new("validators");
