use crate::error::ContractError;
use crate::state::{ADDRESSES, ADDRESS_LIST};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, Env, Order, StdResult};
use cw_ownable::get_ownership;
use cw_storage_plus::Bound;
use ve3_shared::constants::{DEFAULT_LIMIT, MAX_LIMIT};
use ve3_shared::error::SharedError;
use ve3_shared::msgs_global_config::{AddressListResponse, AddressResponse, QueryMsg};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
  match msg {
    QueryMsg::Address(address_type) => Ok(to_json_binary(&query_address(deps, address_type)?)?),
    QueryMsg::Addresses(address_types) => {
      Ok(to_json_binary(&query_addresses(deps, address_types)?)?)
    },
    QueryMsg::AllAddresses {
      start_after,
      limit,
    } => Ok(to_json_binary(&query_all_addresses(deps, start_after, limit)?)?),

    QueryMsg::Ownership {} => Ok(to_json_binary(&get_ownership(deps.storage)?)?),
    QueryMsg::AddressList(address_type) => {
      Ok(to_json_binary(&query_address_list(deps, address_type)?)?)
    },
  }
}

fn query_address(deps: Deps, address_type: String) -> StdResult<AddressResponse> {
  let address = ADDRESSES.load(deps.storage, address_type.clone())?;
  Ok((address_type, address))
}

fn query_addresses(deps: Deps, address_types: Vec<String>) -> StdResult<Vec<AddressResponse>> {
  address_types
    .into_iter()
    .map(|address_type| query_address(deps, address_type))
    .collect::<StdResult<Vec<_>>>()
}

fn query_address_list(
  deps: Deps,
  address_type: String,
) -> Result<AddressListResponse, ContractError> {
  let address_list = ADDRESS_LIST
    .load(deps.storage, address_type.clone())
    .map_err(|_| SharedError::NotFound(format!("address type: {0}", address_type).to_string()))?;
  Ok(address_list)
}

fn query_all_addresses(
  deps: Deps,
  start_after: Option<String>,
  limit: Option<u32>,
) -> StdResult<Vec<AddressResponse>> {
  let start = start_after.map(Bound::exclusive);
  let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

  ADDRESSES
    .range(deps.storage, start, None, Order::Ascending)
    .take(limit)
    .map(|item| {
      let (k, v) = item?;
      Ok((k, v))
    })
    .collect()
}
