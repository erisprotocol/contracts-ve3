use crate::{
  constants::{CONTRACT_NAME, CONTRACT_VERSION},
  error::ContractError,
  state::{ROUTES, TOKEN_CONFIG},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, Response};
use cw2::{get_contract_version, set_contract_version};
use ve3_shared::{error::SharedError, msgs_global_config::MigrateMsg};

/// Manages contract migration
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
  let contract_version = get_contract_version(deps.storage)?;
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  if contract_version.contract != CONTRACT_NAME {
    return Err(
      SharedError::ContractMismatch(contract_version.contract, CONTRACT_VERSION.to_string()).into(),
    );
  }

  if msg.clear == Some(true) {
    // clear routes
    ROUTES.clear(deps.storage);
    TOKEN_CONFIG.clear(deps.storage);
  }

  Ok(
    Response::new()
      .add_attribute("previous_contract_name", &contract_version.contract)
      .add_attribute("previous_contract_version", &contract_version.version)
      .add_attribute("new_contract_name", CONTRACT_NAME)
      .add_attribute("new_contract_version", CONTRACT_VERSION),
  )
}
