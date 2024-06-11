use cw_storage_plus::Item;
use ve3_shared::msgs_connector_emission::Config;

pub const CONFIG: Item<Config> = Item::new("config");
