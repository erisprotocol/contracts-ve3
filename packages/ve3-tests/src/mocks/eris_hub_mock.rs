use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
  entry_point, to_json_binary, Binary, Coin, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
  StdResult, Uint128,
};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ve3_shared::error::SharedError;
pub type ContractResult = Result<Response, SharedError>;

#[cw_serde]
pub enum ExecuteMsg {
  UpdateExchangeRate {
    exchange_rate: Decimal,
  },
}

#[cw_serde]
pub struct InstantiateMsg {
  pub exchange_rate: Decimal,
}

#[cw_serde]
pub enum QueryMsg {
  State {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct StateResponse {
  /// Total supply to the Stake token
  pub total_ustake: Uint128,
  /// Total amount of uluna staked (bonded)
  pub total_uluna: Uint128,
  /// The exchange rate between ustake and uluna, in terms of uluna per ustake
  pub exchange_rate: Decimal,
  /// Staking rewards currently held by the contract that are ready to be reinvested
  pub unlocked_coins: Vec<Coin>,
  // Amount of uluna currently unbonding
  pub unbonding: Uint128,
  // Amount of uluna currently available as balance of the contract
  pub available: Uint128,
  // Total amount of uluna within the contract (bonded + unbonding + available)
  pub tvl_uluna: Uint128,
}

#[cw_serde]
pub struct Config {
  pub exchange_rate: Decimal,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[entry_point]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> ContractResult {
  CONFIG.save(
    deps.storage,
    &Config {
      exchange_rate: msg.exchange_rate,
    },
  )?;
  Ok(Response::new())
}

#[entry_point]
pub fn execute(deps: DepsMut, _env: Env, _info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
  match msg {
    ExecuteMsg::UpdateExchangeRate {
      exchange_rate,
    } => {
      CONFIG.save(
        deps.storage,
        &Config {
          exchange_rate,
        },
      )?;
      Ok(Response::new())
    },
  }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
  match msg {
    QueryMsg::State {} => Ok(to_json_binary(&StateResponse {
      total_ustake: Uint128::zero(),
      total_uluna: Uint128::zero(),
      exchange_rate: CONFIG.load(deps.storage)?.exchange_rate,
      unlocked_coins: vec![],
      unbonding: Uint128::zero(),
      available: Uint128::zero(),
      tvl_uluna: Uint128::zero(),
    })?),
  }
}
