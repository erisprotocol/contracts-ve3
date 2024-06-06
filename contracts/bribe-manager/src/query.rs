use crate::state::CONFIG;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, Env, StdResult};
use ve3_shared::contract_bribe_manager::QueryMsg;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
  match msg {
    QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
  }
}
