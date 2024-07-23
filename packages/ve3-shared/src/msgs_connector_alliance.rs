use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
  to_json_binary, Addr, CosmosMsg, Decimal, QuerierWrapper, StdResult, Uint128, WasmMsg,
};
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked};
#[allow(unused_imports)]
use std::collections::HashSet;

use crate::{
  adapters::{asset_gauge::AssetGauge, global_config_adapter::ConfigExt},
  constants::AT_ASSET_GAUGE,
  error::SharedError,
};

#[cw_serde]
pub struct InstantiateMsg {
  pub reward_denom: String,
  pub zasset_denom: String,
  pub alliance_token_denom: String,
  pub global_config_addr: String,
  pub gauge: String,

  pub lst_hub_address: String,
  pub lst_asset_info: AssetInfoUnchecked,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum CallbackMsg {
  ClaimRewardsCallback {},
  BondRewardsCallback {
    initial: Asset,
    receiver: Addr,
  },
}

impl CallbackMsg {
  pub fn into_cosmos_msg(&self, contract_addr: &Addr) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: contract_addr.to_string(),
      msg: to_json_binary(&ExecuteMsg::Callback(self.clone()))?,
      funds: vec![],
    }))
  }
}

#[cw_serde]
pub struct AllianceDelegateMsg {
  pub delegations: Vec<AllianceDelegation>,
}

#[cw_serde]
pub struct AllianceUndelegateMsg {
  pub undelegations: Vec<AllianceDelegation>,
}

#[cw_serde]
pub struct AllianceDelegation {
  pub validator: String,
  pub amount: Uint128,
}

#[cw_serde]
pub struct AllianceRedelegation {
  pub src_validator: String,
  pub dst_validator: String,
  pub amount: Uint128,
}

#[cw_serde]
pub struct AllianceRedelegateMsg {
  pub redelegations: Vec<AllianceRedelegation>,
}

#[cw_serde]
pub struct Config {
  pub global_config_addr: Addr,

  pub reward_denom: String,
  pub zasset_denom: String,
  pub alliance_token_denom: String,
  pub alliance_token_supply: Uint128,
  pub gauge: String,

  pub lst_hub_addr: Addr,
  pub lst_asset_info: AssetInfo,
}

impl Config {
  pub fn asset_gauge(&self, querier: &QuerierWrapper) -> Result<AssetGauge, SharedError> {
    self.global_config().get_address(querier, AT_ASSET_GAUGE).map(AssetGauge)
  }
}

#[cw_serde]
pub struct State {
  pub last_exchange_rate: Decimal,
  pub taken: Uint128,
  pub harvested: Uint128,
}

#[cw_serde]
pub enum ExecuteMsg {
  DistributeRebase {
    update: Option<bool>,
  },

  Withdraw {
    recipient: Option<String>,
  },

  // Privileged functions
  ClaimRewards {},

  AllianceDelegate(AllianceDelegateMsg),
  AllianceUndelegate(AllianceUndelegateMsg),
  AllianceRedelegate(AllianceRedelegateMsg),

  RemoveValidator {
    validator: String,
  },

  Callback(CallbackMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
  #[returns(Config)]
  Config {},

  #[returns(StateResponse)]
  State {},

  #[returns(HashSet<Addr>)]
  Validators {},
}

#[cw_serde]
pub struct StateResponse {
  pub last_exchange_rate: Decimal,
  pub taken: Uint128,
  pub harvested: Uint128,

  pub total_shares: Uint128,
  pub stake_available: Uint128,
  pub stake_in_contract: Uint128,

  pub share_exchange_rate: Decimal,
}
