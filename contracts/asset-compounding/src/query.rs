use crate::error::ContractError;
use crate::state::CONFIG;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, Env};
use ve3_shared::msgs_connector_emission::QueryMsg;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
  match msg {
    QueryMsg::Config {} => Ok(to_json_binary(&CONFIG.load(deps.storage)?)?),
  }
}
