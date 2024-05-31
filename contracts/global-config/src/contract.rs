use crate::{
    error::ContractResult,
    global_config_adapter::{ADDRESSES, ADDRESS_LIST},
    msg::{AddressListResponse, AddressResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw_ownable::{get_ownership, update_ownership};
use cw_storage_plus::Bound;
use ve3_shared::constants::{DEFAULT_LIMIT, MAX_LIMIT};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult {
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
    match msg {
        ExecuteMsg::UpdateOwnership(action) => {
            update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::new().add_attribute("action", "update_ownership"))
        },
        ExecuteMsg::SetAddresses {
            addresses,
        } => set_addresses(deps, info.sender, addresses),
        ExecuteMsg::SetAdressList {
            address_type,
            addresses,
        } => set_address_list(deps, info.sender, address_type, addresses),
    }
}

fn set_addresses(deps: DepsMut, sender: Addr, addresses: Vec<(String, String)>) -> ContractResult {
    cw_ownable::assert_owner(deps.storage, &sender)?;

    for (address_type, address) in addresses {
        if address.is_empty() {
            ADDRESSES.remove(deps.storage, address_type);
        } else {
            ADDRESSES.save(deps.storage, address_type, &deps.api.addr_validate(&address)?)?;
        }
    }

    Ok(Response::new().add_attribute("action", "set_addresses"))
}

fn set_address_list(
    deps: DepsMut,
    sender: Addr,
    address_type: String,
    addresses: Vec<String>,
) -> ContractResult {
    cw_ownable::assert_owner(deps.storage, &sender)?;

    let mut addresses_addr = vec![];
    for address in addresses {
        addresses_addr.push(deps.api.addr_validate(&address)?);
    }

    ADDRESS_LIST.save(deps.storage, address_type, &addresses_addr)?;

    Ok(Response::new().add_attribute("action", "set_address_list"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Address(address_type) => to_json_binary(&query_address(deps, address_type)?),
        QueryMsg::Addresses(address_types) => {
            to_json_binary(&query_addresses(deps, address_types)?)
        },
        QueryMsg::AllAddresses {
            start_after,
            limit,
        } => to_json_binary(&query_all_addresses(deps, start_after, limit)?),

        QueryMsg::Ownership {} => to_json_binary(&get_ownership(deps.storage)?),
        QueryMsg::AddressList(address_type) => {
            to_json_binary(&query_address_list(deps, address_type)?)
        },
    }
}

fn query_address(deps: Deps, address_type: String) -> StdResult<AddressResponse> {
    let address = ADDRESSES.load(deps.storage, address_type.clone())?;
    Ok((address_type, address))
}

fn query_address_list(deps: Deps, address_type: String) -> StdResult<AddressListResponse> {
    let address_list = ADDRESS_LIST.load(deps.storage, address_type.clone())?;
    Ok((address_type, address_list))
}

fn query_addresses(deps: Deps, address_types: Vec<String>) -> StdResult<Vec<AddressResponse>> {
    address_types
        .into_iter()
        .map(|address_type| query_address(deps, address_type))
        .collect::<StdResult<Vec<_>>>()
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult {
    Ok(Response::default())
}

#[cfg(test)]
mod tests {}
