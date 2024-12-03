use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
  coin, to_json_binary, Addr, CosmosMsg, Decimal, QuerierWrapper, StdResult, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use cw_asset::Asset;
use serde::{Deserialize, Serialize};

use crate::error::SharedError;

pub struct ErisHub<'a>(pub &'a Addr);

#[cw_serde]
pub enum QueryMsg {
  State {},
}

#[cw_serde]
pub enum ExecuteMsg {
  Bond {
    receiver: Option<String>,
  },
}

#[derive(Deserialize, Serialize)]
pub struct StateResponse {
  /// The exchange rate between ustake and uluna, in terms of uluna per ustake
  pub exchange_rate: Decimal,
}

impl<'a> ErisHub<'a> {
  pub fn bond_msg(&self, asset: Asset, receiver: Option<String>) -> Result<CosmosMsg, SharedError> {
    match asset.info {
      cw_asset::AssetInfoBase::Native(denom) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: self.0.to_string(),
        msg: to_json_binary(&ExecuteMsg::Bond {
          receiver,
        })?,
        funds: vec![coin(asset.amount.u128(), denom)],
      })),
      cw_asset::AssetInfoBase::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Send {
          contract: self.0.to_string(),
          amount: asset.amount,
          msg: to_json_binary(&ExecuteMsg::Bond {
            receiver,
          })?,
        })?,
        funds: vec![],
      })),
      _ => Err(SharedError::NotSupported("only native supported".to_string())),
    }
  }

  pub fn query_exchange_rate(&self, querier: &QuerierWrapper) -> StdResult<Decimal> {
    let result: StateResponse =
      querier.query_wasm_smart(self.0.to_string(), &QueryMsg::State {})?;
    Ok(result.exchange_rate)
  }
}
