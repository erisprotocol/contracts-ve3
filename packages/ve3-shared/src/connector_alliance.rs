use std::collections::HashSet;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, StdResult, Uint128, WasmMsg};
use cw_asset::AssetInfo;

#[cw_serde]
pub struct InstantiateMsg {
    pub alliance_token_denom: String,
    pub reward_denom: String,
    pub global_config_addr: String,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
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
pub enum CallbackMsg {
    ClaimRewardsCallback {
        asset: AssetInfo,
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
    pub alliance_token_denom: String,
    pub alliance_token_supply: Uint128,
    pub reward_denom: String,
    pub global_config_addr: Addr,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},

    #[returns(HashSet<Addr>)]
    Validators {},
}
