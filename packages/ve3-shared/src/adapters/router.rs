use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, QuerierWrapper, Uint128};
use cw_asset::{Asset, AssetInfo};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::SharedError;

use super::pair::OldAssetInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Router(pub Addr);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RouterQueryMsg {
  SimulateSwapOperations {
    /// The amount of tokens to swap
    offer_amount: Uint128,
    /// The swap operations to perform, each swap involving a specific pool
    operations: Vec<SwapOperation>,
  },
}

#[cw_serde]
pub struct SimulateSwapOperationsResponse {
  /// The amount of tokens received in a swap simulation
  pub amount: Uint128,
}

#[cw_serde]
pub enum SwapOperation {
  /// Native swap
  NativeSwap {
    /// The name (denomination) of the native asset to swap from
    offer_denom: String,
    /// The name (denomination) of the native asset to swap to
    ask_denom: String,
  },
  /// ASTRO swap
  AstroSwap {
    /// Information about the asset being swapped
    offer_asset_info: OldAssetInfo,
    /// Information about the asset we swap to
    ask_asset_info: OldAssetInfo,
  },
}

impl Router {
  pub fn query_simulate(
    &self,
    querier: &QuerierWrapper,
    offer_asset: Asset,
    path: Vec<AssetInfo>,
  ) -> Result<SimulateSwapOperationsResponse, SharedError> {
    let mut operations = vec![SwapOperation::AstroSwap {
      offer_asset_info: OldAssetInfo::from_new(offer_asset.info)?,
      ask_asset_info: OldAssetInfo::from_new(path[0].clone())?,
    }];

    for i in 0..(path.len() - 1) {
      let current = path[i].clone();
      let next = path[i + 1].clone();
      operations.push(SwapOperation::AstroSwap {
        offer_asset_info: OldAssetInfo::from_new(current)?,
        ask_asset_info: OldAssetInfo::from_new(next)?,
      })
    }

    let response: SimulateSwapOperationsResponse = querier.query_wasm_smart(
      self.0.to_string(),
      &RouterQueryMsg::SimulateSwapOperations {
        offer_amount: offer_asset.amount,
        operations,
      },
    )?;

    Ok(response)
  }
}
