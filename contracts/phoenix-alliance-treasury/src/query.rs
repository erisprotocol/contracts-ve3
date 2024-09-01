#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, Env};
use ve3_shared::msgs_phoenix_alliance_treasury::QueryMsg;

use crate::{
  error::ContractError,
  state::{CONFIG, STATE, VALIDATORS},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
  match msg {
    QueryMsg::Config {} => get_config(deps),
    QueryMsg::State {} => get_state(deps, env),
    QueryMsg::Validators {} => get_validators(deps),
  }
}

fn get_config(deps: Deps) -> Result<Binary, ContractError> {
  let res = CONFIG.load(deps.storage)?;
  Ok(to_json_binary(&res)?)
}

fn get_state(deps: Deps, _env: Env) -> Result<Binary, ContractError> {
  let res = STATE.load(deps.storage)?;
  Ok(to_json_binary(&res)?)
}

fn get_validators(deps: Deps) -> Result<Binary, ContractError> {
  let res = VALIDATORS.load(deps.storage)?;
  Ok(to_json_binary(&res)?)
}
