use crate::error::SharedError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Uint128, WasmMsg};

#[cw_serde]
pub enum ExecuteMsg {
  Mint {
    amount: Uint128,
  },
}

pub struct MintProxy(pub Addr);

impl MintProxy {
  pub fn mint_msg(&self, amount: Uint128) -> Result<CosmosMsg, SharedError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: self.0.to_string(),
      msg: to_json_binary(&ExecuteMsg::Mint {
        amount,
      })?,
      funds: vec![],
    }))
  }
}
