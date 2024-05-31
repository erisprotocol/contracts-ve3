use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coins, to_json_binary, Addr, CosmosMsg, Uint128, WasmMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_asset::Asset;

use crate::error::SharedError;

#[cw_serde]
pub enum AstroportExecuteMsg {
    ClaimRewards {
        /// The LP token cw20 address or token factory denom
        lp_tokens: Vec<String>,
    },
    /// Stake LP tokens in the Generator. LP tokens staked on behalf of recipient if recipient is set.
    /// Otherwise LP tokens are staked on behalf of message sender.
    Deposit {
        recipient: Option<String>,
    },
    /// Withdraw LP tokens from the Generator
    Withdraw {
        /// The LP token cw20 address or token factory denom
        lp_token: String,
        /// The amount to withdraw. Must not exceed total staked amount.
        amount: Uint128,
    },
    /// Receives a message of type [`Cw20ReceiveMsg`]. Handles cw20 LP token deposits.
    Receive(Cw20ReceiveMsg),
}

#[cw_serde]
/// Cw20 hook message template
pub enum Cw20Msg {
    Deposit {
        recipient: Option<String>,
    },
    /// Besides this enum variant is redundant we keep this for backward compatibility with old pair contracts
    DepositFor(String),
}

pub struct AstroportIncentives(pub Addr);

impl AstroportIncentives {
    pub fn claim_rewards_msg(&self, lp_tokens: Vec<String>) -> Result<CosmosMsg, SharedError> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_json_binary(&AstroportExecuteMsg::ClaimRewards {
                lp_tokens,
            })?,
            funds: vec![],
        }))
    }

    pub fn deposit(&self, asset: Asset) -> Result<CosmosMsg, SharedError> {
        match asset.info {
            cw_asset::AssetInfoBase::Native(native) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: self.0.to_string(),
                msg: to_json_binary(&AstroportExecuteMsg::Deposit {
                    recipient: None,
                })?,
                funds: coins(asset.amount.u128(), native),
            })),
            cw_asset::AssetInfoBase::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                funds: vec![],
                msg: to_json_binary(&Cw20ExecuteMsg::Send {
                    contract: self.0.to_string(),
                    amount: asset.amount,
                    msg: to_json_binary(&Cw20Msg::Deposit {
                        recipient: None,
                    })?,
                })?,
            })),
            _ => Err(SharedError::NotSupported("asset type".to_string())),
        }
    }

    pub fn withdraw(&self, asset: Asset) -> Result<CosmosMsg, SharedError> {
        match asset.info {
            cw_asset::AssetInfoBase::Native(native) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: self.0.to_string(),
                msg: to_json_binary(&AstroportExecuteMsg::Withdraw {
                    lp_token: native,
                    amount: asset.amount,
                })?,
                funds: vec![],
            })),
            cw_asset::AssetInfoBase::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: self.0.to_string(),
                msg: to_json_binary(&AstroportExecuteMsg::Withdraw {
                    lp_token: contract_addr.to_string(),
                    amount: asset.amount,
                })?,
                funds: vec![],
            })),
            _ => Err(SharedError::NotSupported("asset type".to_string())),
        }
    }
}
