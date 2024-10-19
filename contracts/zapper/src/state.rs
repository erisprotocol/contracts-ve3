use cosmwasm_schema::cw_serde;
use cw_asset::AssetInfo;
use cw_storage_plus::{Item, Map};
use ve3_shared::msgs_zapper::{Config, Stage, StageType};

#[cw_serde]
pub struct RouteConfig {
  pub stages: Vec<Stage>,
}

pub const ROUTES: Map<(String, String), RouteConfig> = Map::new("routes");
pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub enum TokenConfig {
  TargetSwap,
  TargetPair(StageType),
}
pub const TOKEN_CONFIG: Map<&AssetInfo, TokenConfig> = Map::new("token_config");
