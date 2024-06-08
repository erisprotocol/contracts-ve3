use crate::{
  constants::{CONTRACT_NAME, CONTRACT_VERSION},
  error::ContractResult,
  state::{ADDRESSES, ADDRESS_LIST},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_string, Addr, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use cw_ownable::update_ownership;
use ve3_shared::msgs_global_config::{ExecuteMsg, InstantiateMsg};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> ContractResult {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
  cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;
  Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
  match msg {
    ExecuteMsg::UpdateOwnership(action) => {
      update_ownership(deps, &env.block, &info.sender, action.clone())?;
      Ok(
        Response::new()
          .add_attribute("action", "update_ownership")
          .add_attribute("data", to_json_string(&action)?),
      )
    },
    ExecuteMsg::SetAddresses {
      addresses,
      lists,
    } => set_addresses(deps, info.sender, addresses, lists),
    ExecuteMsg::ClearAddresses {
      addresses,
    } => clear_addresses(deps, info.sender, addresses),
    ExecuteMsg::ClearLists {
      lists,
    } => clear_lists(deps, info.sender, lists),
  }
}

fn set_addresses(
  mut deps: DepsMut,
  sender: Addr,
  addresses: Vec<(String, String)>,
  lists: Vec<(String, Vec<String>)>,
) -> ContractResult {
  cw_ownable::assert_owner(deps.storage, &sender)?;

  for (address_type, address) in addresses {
    if address.is_empty() {
      ADDRESSES.remove(deps.storage, address_type);
    } else {
      ADDRESSES.save(deps.storage, address_type, &deps.api.addr_validate(&address)?)?;
    }
  }

  for (address_type, list) in lists {
    _set_address_list(&mut deps, address_type, list)?;
  }

  Ok(Response::new().add_attribute("action", "set_addresses"))
}

fn clear_addresses(deps: DepsMut, sender: Addr, addresses: Vec<String>) -> ContractResult {
  cw_ownable::assert_owner(deps.storage, &sender)?;

  for address_type in addresses {
    ADDRESSES.remove(deps.storage, address_type);
  }

  Ok(Response::new().add_attribute("action", "clear_addresses"))
}

fn clear_lists(deps: DepsMut, sender: Addr, addresses: Vec<String>) -> ContractResult {
  cw_ownable::assert_owner(deps.storage, &sender)?;

  for address_type in addresses {
    ADDRESS_LIST.remove(deps.storage, address_type);
  }

  Ok(Response::new().add_attribute("action", "clear_lists"))
}

fn _set_address_list(deps: &mut DepsMut, address_type: String, list: Vec<String>) -> StdResult<()> {
  let mut addresses_addr = vec![];
  for address in list {
    addresses_addr.push(deps.api.addr_validate(&address)?);
  }

  ADDRESS_LIST.save(deps.storage, address_type, &addresses_addr)?;
  Ok(())
}
