use cw_asset::AssetInfo;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use ve3_shared::msgs_asset_compounding::{CompoundingAssetConfig, Config, ExchangeHistory};

pub const CONFIG: Item<Config> = Item::new("config");
pub const TOKEN_INDEX: Item<u64> = Item::new("token_index");
pub const EXCHANGE_HISTORY: Map<(&str, &AssetInfo, u64), ExchangeHistory> =
  Map::new("exchange_history");

pub(crate) struct AssetConfigIndexes<'a> {
  pub by_denom: MultiIndex<'a, String, CompoundingAssetConfig, (&'a str, &'a AssetInfo)>,
}

impl IndexList<CompoundingAssetConfig> for AssetConfigIndexes<'_> {
  fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<CompoundingAssetConfig>> + '_> {
    let v: Vec<&dyn Index<CompoundingAssetConfig>> = vec![&self.by_denom];
    Box::new(v.into_iter())
  }
}

/// (gauge, asset) -> CompoundingAssetConfig
pub(crate) fn asset_config_map<'a>(
) -> IndexedMap<'a, (&'a str, &'a AssetInfo), CompoundingAssetConfig, AssetConfigIndexes<'a>> {
  IndexedMap::new(
    "asset_config",
    AssetConfigIndexes {
      by_denom: MultiIndex::new(
        |_, d: &CompoundingAssetConfig| d.amp_denom.clone(),
        "asset_config",
        "asset_config__amp_denom",
      ),
    },
  )
}
