use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, WasmMsg};

use crate::{connector_alliance, error::SharedError};

pub struct Connector(pub Addr);

impl Connector {
    pub fn claim_rewards_msg(&self) -> Result<CosmosMsg, SharedError> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_json_binary(&connector_alliance::ExecuteMsg::ClaimRewards {})?,
            funds: vec![],
        }))
    }
}
