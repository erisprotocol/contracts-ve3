use crate::{
  error::SharedError,
  extensions::{asset_ext::AssetExt, asset_info_ext::AssetInfoExt},
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
  to_json_binary, Addr, Coin, CosmosMsg, Decimal, QuerierWrapper, StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use cw_asset::{Asset, AssetError, AssetInfo, AssetInfoBase};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Pair(pub Addr);

#[cw_serde]
pub enum OldAssetInfo {
  /// Non-native Token
  Token {
    contract_addr: Addr,
  },
  /// Native token
  NativeToken {
    denom: String,
  },
}

impl OldAssetInfo {
  pub fn to_new(&self) -> AssetInfo {
    match self {
      OldAssetInfo::Token {
        contract_addr,
      } => cw_asset::AssetInfoBase::Cw20(contract_addr.clone()),
      OldAssetInfo::NativeToken {
        denom,
      } => cw_asset::AssetInfoBase::Native(denom.clone()),
    }
  }

  pub fn from_new(info: AssetInfo) -> Result<OldAssetInfo, SharedError> {
    Ok(match info {
      AssetInfoBase::Native(denom) => OldAssetInfo::NativeToken {
        denom,
      },
      AssetInfoBase::Cw20(contract_addr) => OldAssetInfo::Token {
        contract_addr,
      },
      _ => Err(SharedError::NotSupportedAssetInfo())?,
    })
  }
}

#[cw_serde]
pub struct OldAsset {
  /// Information about an asset stored in a [`AssetInfo`] struct
  pub info: OldAssetInfo,
  /// A token amount
  pub amount: Uint128,
}

impl OldAsset {
  pub fn from_new(asset: Asset) -> Result<OldAsset, SharedError> {
    Ok(OldAsset {
      info: OldAssetInfo::from_new(asset.info)?,
      amount: asset.amount,
    })
  }
}

#[cw_serde]
pub struct PairInfoAstroport {
  /// Asset information for the assets in the pool
  pub asset_infos: Vec<OldAssetInfo>,
  /// Pair contract address
  pub contract_addr: Addr,
  /// Pair LP token address
  pub liquidity_token: String,
  /// The pool type (xyk, stableswap etc) available in [`PairType`]
  pub pair_type: PairType,
}

#[cw_serde]
pub struct PairInfo {
  /// Asset information for the assets in the pool
  pub asset_infos: Vec<AssetInfo>,
  /// Pair contract address
  pub contract_addr: Addr,
  /// Pair LP token address
  pub liquidity_token: AssetInfo,
  /// The pool type (xyk, stableswap etc) available in [`PairType`]
  pub pair_type: PairType,
}

impl PairInfo {
  pub fn query_pools(
    &self,
    querier: &QuerierWrapper,
    address: &Addr,
  ) -> Result<Vec<Asset>, AssetError> {
    self
      .asset_infos
      .iter()
      .map(|a| a.with_balance_query(querier, address))
      .collect::<Result<Vec<_>, AssetError>>()
  }
}

#[cw_serde]
pub enum PairType {
  /// XYK pair type
  Xyk {},
  /// Stable pair type
  Stable {},
  /// Custom pair type
  Custom(String),

  /// Stable pair type
  StableWhiteWhale {},
  /// XYK pair type
  XykWhiteWhale {},
}

#[cw_serde]
pub enum PairExecuteMsg {
  Swap {
    offer_asset: OldAsset,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    to: Option<String>,
  },
  ProvideLiquidity {
    assets: Vec<OldAsset>,
    slippage_tolerance: Option<Decimal>,
    receiver: Option<String>,
  },
  WithdrawLiquidity {},
}

#[cw_serde]
pub struct PairInfoWw {
  pub asset_infos: [OldAssetInfo; 2],
  pub contract_addr: String,
  pub liquidity_token: OldAssetInfo,
  pub asset_decimals: [u8; 2],
  pub pair_type: PairTypeWw,
}

#[cw_serde]
pub enum PairTypeWw {
  StableSwap {
    /// The amount of amplification to perform on the constant product part of the swap formula.
    amp: u64,
  },
  ConstantProduct,
}

#[cw_serde]
pub enum PairCw20HookMsg {
  /// Swap a given amount of asset
  Swap {
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    to: Option<String>,
  },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PairQueryMsg {
  /// Returns information about a pair in an object of type [`super::asset::PairInfo`].
  Pair {},
  //   /// Returns information about a pool in an object of type [`PoolResponse`].
  //   Pool {},
  //   /// Returns contract configuration settings in a custom [`ConfigResponse`] structure.
  //   Config {},
  Simulation {
    offer_asset: OldAsset,
    ask_asset_info: Option<OldAssetInfo>,
  },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SimulationResponse {
  pub return_amount: Uint128,
  pub spread_amount: Uint128,
  pub commission_amount: Uint128,
}

impl Pair {
  pub fn query_astroport_pair_info(&self, querier: &QuerierWrapper) -> StdResult<PairInfo> {
    let pair: PairInfoAstroport =
      querier.query_wasm_smart(self.0.to_string(), &PairQueryMsg::Pair {})?;

    Ok(PairInfo {
      asset_infos: pair.asset_infos.iter().map(|a| a.to_new()).collect::<Vec<_>>(),
      contract_addr: pair.contract_addr,
      pair_type: pair.pair_type,
      liquidity_token: if pair.liquidity_token.starts_with("factory/") {
        AssetInfo::native(pair.liquidity_token)
      } else {
        AssetInfo::cw20(Addr::unchecked(pair.liquidity_token))
      },
    })
  }

  pub fn query_simulate(
    &self,
    querier: &QuerierWrapper,
    offer_asset: Asset,
    ask_asset_info: Option<AssetInfo>,
  ) -> Result<SimulationResponse, SharedError> {
    let response: SimulationResponse = querier.query_wasm_smart(
      self.0.to_string(),
      &PairQueryMsg::Simulation {
        ask_asset_info: if let Some(ask_asset_info) = ask_asset_info {
          Some(OldAssetInfo::from_new(ask_asset_info)?)
        } else {
          None
        },
        offer_asset: OldAsset::from_new(offer_asset)?,
      },
    )?;

    Ok(response)
  }

  pub fn query_ww_pair_info(&self, querier: &QuerierWrapper) -> StdResult<PairInfo> {
    let pair: PairInfoWw = querier.query_wasm_smart(self.0.to_string(), &PairQueryMsg::Pair {})?;
    Ok(PairInfo {
      asset_infos: pair.asset_infos.iter().map(|a| a.to_new()).collect::<Vec<_>>(),
      contract_addr: Addr::unchecked(pair.contract_addr),
      liquidity_token: pair.liquidity_token.to_new(),
      pair_type: match pair.pair_type {
        PairTypeWw::StableSwap {
          ..
        } => PairType::StableWhiteWhale {},
        PairTypeWw::ConstantProduct => PairType::XykWhiteWhale {},
      },
    })
  }

  /// Generate msg for swapping specified asset
  pub fn swap_msg(
    &self,
    asset: &Asset,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    to: Option<String>,
  ) -> Result<CosmosMsg, SharedError> {
    let wasm_msg = match &asset.info {
      AssetInfoBase::Cw20(contract_addr) => WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Send {
          contract: self.0.to_string(),
          amount: asset.amount,
          msg: to_json_binary(&PairCw20HookMsg::Swap {
            belief_price,
            max_spread,
            to,
          })?,
        })?,
        funds: vec![],
      },

      AssetInfoBase::Native(denom) => WasmMsg::Execute {
        contract_addr: self.0.to_string(),
        msg: to_json_binary(&PairExecuteMsg::Swap {
          offer_asset: OldAsset {
            info: OldAssetInfo::NativeToken {
              denom: denom.to_string(),
            },
            amount: asset.amount,
          },
          belief_price,
          max_spread,
          to,
        })?,
        funds: vec![Coin {
          denom: denom.clone(),
          amount: asset.amount,
        }],
      },
      _ => Err(SharedError::NotSupported("asset info type".to_string()))?,
    };

    Ok(CosmosMsg::Wasm(wasm_msg))
  }

  pub fn provide_liquidity_msg(
    &self,
    assets: Vec<Asset>,
    slippage_tolerance: Option<Decimal>,
    receiver: Option<String>,
    mut funds: Vec<Coin>,
  ) -> Result<CosmosMsg, SharedError> {
    funds.sort_by(|a, b| a.denom.cmp(&b.denom));
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: self.0.to_string(),
      msg: to_json_binary(&PairExecuteMsg::ProvideLiquidity {
        assets: assets
          .into_iter()
          .map(OldAsset::from_new)
          .collect::<Result<Vec<_>, SharedError>>()?,
        slippage_tolerance,
        receiver,
      })?,
      funds,
    }))
  }
  pub fn withdraw_liquidity_msg(&self, lp: Asset) -> Result<CosmosMsg, SharedError> {
    lp.send_or_execute_msg(self.0.to_string(), &PairExecuteMsg::WithdrawLiquidity {})
  }
}
