use cosmwasm_std::Addr;
use cw_asset::AssetInfo;
use cw_storage_plus::{Item, Map};
use std::collections::HashSet;
use ve3_shared::msgs_phoenix_treasury::{Config, Oracle, State, TreasuryAction};

pub const CONFIG: Item<Config> = Item::new("config");
pub const VALIDATORS: Item<HashSet<String>> = Item::new("validators");
pub const STATE: Item<State> = Item::new("state");

pub const ACTIONS: Map<u64, TreasuryAction> = Map::new("actions");
pub const USER_ACTIONS: Map<(&Addr, u64), ()> = Map::new("user_actions");
pub const ORACLES: Map<&AssetInfo, Oracle<Addr>> = Map::new("oracles");
