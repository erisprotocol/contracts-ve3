#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, Env, StdResult};
use ve3_shared::msgs_connector_alliance::QueryMsg;

use crate::state::{CONFIG, STATE, VALIDATORS};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
  Ok(match msg {
    QueryMsg::Config {} => get_config(deps)?,
    QueryMsg::State {} => get_state(deps)?,
    QueryMsg::Validators {} => get_validators(deps)?,
  })
}

fn get_config(deps: Deps) -> StdResult<Binary> {
  let res = CONFIG.load(deps.storage)?;
  to_json_binary(&res)
}

fn get_state(deps: Deps) -> StdResult<Binary> {
  let res = STATE.load(deps.storage)?;
  to_json_binary(&res)
}

fn get_validators(deps: Deps) -> StdResult<Binary> {
  let validators = VALIDATORS.load(deps.storage)?;
  to_json_binary(&validators)
}
