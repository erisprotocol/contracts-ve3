use crate::common::helpers::u;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
  entry_point, to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
  StdResult,
};
use cw_storage_plus::Item;
use ve3_shared::{
  adapters::pair::{PairQueryMsg, SimulationResponse},
  error::SharedError,
};
pub type ContractResult = Result<Response, SharedError>;

#[cw_serde]
pub enum ExecuteMsg {
  UpdatePrice {
    price: Decimal,
  },
}

#[cw_serde]
pub struct InstantiateMsg {
  pub price: Decimal,
}

#[cw_serde]
pub struct Config {
  pub price: Decimal,
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
      price: msg.price,
    },
  )?;
  Ok(Response::new())
}

#[entry_point]
pub fn execute(deps: DepsMut, _env: Env, _info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
  match msg {
    ExecuteMsg::UpdatePrice {
      price,
    } => {
      CONFIG.save(
        deps.storage,
        &Config {
          price,
        },
      )?;
      Ok(Response::new())
    },
  }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: PairQueryMsg) -> StdResult<Binary> {
  match msg {
    PairQueryMsg::Pair {} => todo!(),
    PairQueryMsg::Simulation {
      offer_asset,
      ..
    } => {
      let config = CONFIG.load(deps.storage)?;
      to_json_binary(&SimulationResponse {
        return_amount: offer_asset.amount * config.price,
        spread_amount: u(0),
        commission_amount: Some(u(0)),
        burn_fee_amount: None,
        protocol_fee_amount: None,
        swap_fee_amount: None,
      })
    },
  }
}
