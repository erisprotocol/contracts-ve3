use cosmwasm_std::{Addr, QuerierWrapper};
use cw_ownable::Ownership;
use cw_storage_plus::{Item, Map};

pub const OWNERSHIP: Item<Ownership<Addr>> = Item::new("ownership");
pub const ADDRESSES: Map<String, Addr> = Map::new("addresses");
pub const ADDRESS_LIST: Map<String, Vec<Addr>> = Map::new("address_list");
