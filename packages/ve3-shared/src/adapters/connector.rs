use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, Coin, CosmosMsg, QuerierWrapper, WasmMsg};

use crate::{
  error::SharedError,
  msgs_connector_alliance::{self, Config, QueryMsg},
};

#[cw_serde]
pub struct Connector(pub Addr);

impl Connector {
  pub fn claim_rewards_msg(&self) -> Result<CosmosMsg, SharedError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: self.0.to_string(),
      msg: to_json_binary(&msgs_connector_alliance::ExecuteMsg::ClaimRewards {})?,
      funds: vec![],
    }))
  }

  pub fn withdraw_msg(&self, coin: Coin) -> Result<CosmosMsg, SharedError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: self.0.to_string(),
      msg: to_json_binary(&msgs_connector_alliance::ExecuteMsg::Withdraw {
        recipient: None,
      })?,
      funds: vec![coin],
    }))
  }

  pub fn query_config(&self, querier: &QuerierWrapper) -> Result<Config, SharedError> {
    let assets: Config = querier.query_wasm_smart(self.0.clone(), &QueryMsg::Config {})?;
    Ok(assets)
  }
}
