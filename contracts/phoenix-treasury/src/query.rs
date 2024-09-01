#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, Env, Order};
use cw_storage_plus::Bound;
use ve3_shared::{
  constants::{DEFAULT_LIMIT, MAX_LIMIT},
  msgs_phoenix_treasury::{QueryMsg, TreasuryAction},
};

use crate::{
  error::ContractError,
  state::{ACTIONS, CONFIG, STATE, USER_ACTIONS, VALIDATORS},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
  match msg {
    QueryMsg::Config {} => get_config(deps),
    QueryMsg::State {} => get_state(deps, env),
    QueryMsg::Validators {} => get_validators(deps),
    QueryMsg::Actions {
      limit,
      start_after,
    } => get_actions(deps, start_after, limit),
    QueryMsg::UserActions {
      user,
      limit,
      start_after,
    } => get_user_actions(deps, user, start_after, limit),
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

fn get_actions(
  deps: Deps,
  start_after: Option<u64>,
  limit: Option<u32>,
) -> Result<Binary, ContractError> {
  let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
  let start_after = start_after.map(Bound::exclusive);
  let actions: Vec<TreasuryAction> = ACTIONS
    .range(deps.storage, start_after, None, Order::Ascending)
    .map(|a| {
      let (_, a) = a?;
      Ok(a)
    })
    .take(limit)
    .collect::<Result<Vec<_>, ContractError>>()?;

  Ok(to_json_binary(&actions)?)
}

fn get_user_actions(
  deps: Deps,
  user: String,
  start_after: Option<u64>,
  limit: Option<u32>,
) -> Result<Binary, ContractError> {
  let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
  let start_after = start_after.map(Bound::exclusive);

  let addr = deps.api.addr_validate(&user)?;

  let actions: Vec<TreasuryAction> = USER_ACTIONS
    .prefix(&addr)
    .range(deps.storage, start_after, None, Order::Descending)
    .map(|a| {
      let action = ACTIONS.load(deps.storage, a?.0)?;
      Ok(action)
    })
    .take(limit)
    .collect::<Result<Vec<_>, ContractError>>()?;

  Ok(to_json_binary(&actions)?)
}
