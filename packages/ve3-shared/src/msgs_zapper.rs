use crate::adapters::pair::{Pair, PairInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
  to_json_binary, Addr, Binary, CosmosMsg, QuerierWrapper, StdResult, Uint128, WasmMsg,
};
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked};
#[allow(unused_imports)]
use cw_ownable::{cw_ownable_execute, Ownership};

#[cw_serde]
pub struct InstantiateMsg {
  pub global_config_addr: String,
  pub center_asset_infos: Vec<AssetInfoUnchecked>,
}

#[cw_serde]
pub enum ExecuteMsg {
  CreateLp {
    stage: StageType,
    assets: Vec<AssetInfo>,
    min_received: Option<Uint128>,
    post_action: Option<PostActionCreate>,
  },
  WithdrawLp {
    stage: StageType,
    min_received: Option<Vec<Asset>>,
    post_action: Option<PostActionWithdraw>,
  },

  /// Swaps a number of assets to a single result
  Swap {
    /// LP into which the assets should be compounded into
    into: AssetInfoUnchecked,
    /// List of reward asset send to compound
    assets: Vec<AssetInfo>,
    min_received: Option<Uint128>,
    /// Receiver address for LP token
    receiver: Option<String>,
  },

  Zap {
    into: AssetInfoUnchecked,
    assets: Vec<AssetInfo>,
    min_received: Option<Uint128>,
    post_action: Option<PostActionCreate>,
  },

  UpdateConfig {
    insert_routes: Option<Vec<RouteInit>>,
    delete_routes: Option<Vec<RouteDelete>>,
    update_centers: Option<Vec<AssetInfoUnchecked>>,
  },

  Callback(CallbackMsg),
}

#[cw_serde]
pub struct RouteInit {
  pub routes: Vec<Stage>,
}

#[cw_serde]
pub struct RouteDelete {
  pub from: AssetInfo,
  pub to: AssetInfo,
  pub both: Option<bool>,
}

#[cw_serde]
pub enum CallbackMsg {
  OptimalSwap {
    pair_info: PairInfo,
  },
  SwapStage {
    stage: Stage,
  },
  ProvideLiquidity {
    pair_info: PairInfo,
    receiver: Option<String>,
  },
  AssertReceived {
    asset: Asset,
  },

  Stake {
    token: AssetInfo,
    asset_staking: Addr,
    receiver: String,
  },
  LiquidStake {
    token: AssetInfo,
    compounder: Addr,
    gauge: String,
    receiver: String,
  },

  SendResult {
    token: AssetInfo,
    receiver: String,
  },
  SendResults {
    tokens: Vec<AssetInfo>,
    receiver: String,
    min_received: Option<Vec<Asset>>,
  },
  ExecuteResult {
    token: AssetInfo,
    contract: Addr,
    msg: Binary,
  },
}

impl CallbackMsg {
  pub fn into_cosmos_msg(&self, contract_addr: &Addr) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: String::from(contract_addr),
      msg: to_json_binary(&ExecuteMsg::Callback(self.clone()))?,
      funds: vec![],
    }))
  }
}

#[cw_serde]
pub enum PostActionCreate {
  Stake {
    asset_staking: Addr,
    receiver: Option<String>,
  },
  LiquidStake {
    compounder: Addr,
    gauge: String,
    receiver: Option<String>,
  },
  SendResult {
    receiver: Option<String>,
  },
  ExecuteResult {
    contract: String,
    msg: Binary,
  },
}

#[cw_serde]
pub enum PostActionWithdraw {
  SwapTo {
    asset: AssetInfo,
    min_received: Option<Uint128>,
    receiver: Option<String>,
  },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
  #[returns(Config)]
  Config {},

  // return all known pairs
  #[returns(Vec<RouteResponseItem>)]
  GetRoutes {
    start_after: Option<(AssetInfo, AssetInfo)>,
    limit: Option<u32>,
  },

  // return a single route
  #[returns(RouteResponseItem)]
  GetRoute {
    from: AssetInfo,
    to: AssetInfo,
  },

  #[returns(SupportsSwapResponse)]
  SupportsSwap {
    from: AssetInfo,
    to: AssetInfo,
  },
}

#[cw_serde]
pub struct Config {
  pub global_config_addr: Addr,
  #[serde(default)]
  pub center_asset_infos: Vec<AssetInfo>,
}

#[cw_serde]
pub struct SupportsSwapResponse {
  pub suppored: bool,
}

#[cw_serde]
pub struct RouteResponseItem {
  pub key: (AssetInfo, AssetInfo),
  pub stages: Vec<Stage>,
}

#[cw_serde]
pub struct Stage {
  pub from: AssetInfo,
  pub to: AssetInfo,
  pub stage_type: StageType,
}

#[cw_serde]
pub enum StageType {
  WhiteWhale {
    pair: Addr,
  },
  Astroport {
    pair: Addr,
  },
}

impl StageType {
  pub fn get_pair_info(&self, querier: &QuerierWrapper) -> StdResult<PairInfo> {
    match self {
      StageType::WhiteWhale {
        pair,
      } => Pair(pair.clone()).query_ww_pair_info(querier),
      StageType::Astroport {
        pair,
      } => Pair(pair.clone()).query_astroport_pair_info(querier),
    }
  }
}

#[cw_serde]
pub struct MigrateMsg {}
