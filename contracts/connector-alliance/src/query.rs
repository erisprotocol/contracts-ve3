#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Decimal, Deps, Env};
use cw_asset::AssetInfo;
use ve3_shared::{
  extensions::asset_info_ext::AssetInfoExt,
  msgs_connector_alliance::{QueryMsg, StateResponse},
};

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

fn get_state(deps: Deps, env: Env) -> Result<Binary, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  let state = STATE.load(deps.storage)?;

  
  let stake = config.lst_asset_info;
  let stake_in_contract = stake.query_balance(&deps.querier, env.contract.address)?;
  let stake_available = stake_in_contract.checked_add(state.harvested)?.checked_sub(state.taken)?;

  let zasset = AssetInfo::native(config.zasset_denom.clone());
  let total_shares = zasset.total_supply(&deps.querier)?;

  Ok(to_json_binary(&StateResponse {
    last_exchange_rate: state.last_exchange_rate,
    taken: state.taken,
    harvested: state.harvested,

    stake_in_contract,
    stake_available,
    total_shares,
    share_exchange_rate: if total_shares.is_zero() {
      Decimal::one()
    } else {
      Decimal::from_ratio(stake_available, total_shares)
    },
  })?)
}

fn get_validators(deps: Deps) -> Result<Binary, ContractError> {
  let res = VALIDATORS.load(deps.storage)?;
  Ok(to_json_binary(&res)?)
}
