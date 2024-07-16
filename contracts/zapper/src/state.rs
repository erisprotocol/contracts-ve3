use cosmwasm_schema::cw_serde;
use cw_storage_plus::{Item, Map};
use ve3_shared::msgs_zapper::{Config, Stage};

#[cw_serde]
pub struct RouteConfig {
  pub stages: Vec<Stage>,
}

pub const ROUTES: Map<(String, String), RouteConfig> = Map::new("routes");
pub const CONFIG: Item<Config> = Item::new("config");
