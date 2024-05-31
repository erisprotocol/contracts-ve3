use cosmwasm_std::{to_json_binary, CosmosMsg, Env, StdResult, WasmMsg};
use serde::Serialize;

pub trait EnvExt {
    fn callback_msg<T>(&self, msg: T) -> StdResult<CosmosMsg>
    where
        T: Serialize;
}

impl EnvExt for Env {
    fn callback_msg<T>(&self, msg: T) -> StdResult<CosmosMsg>
    where
        T: Serialize,
    {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.contract.address.to_string(),
            msg: to_json_binary(&msg)?,
            funds: vec![],
        }))
    }
}
