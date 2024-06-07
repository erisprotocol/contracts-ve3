use crate::{
  constants::{CONTRACT_NAME, CONTRACT_VERSION},
  error::ContractResult,
  state::{ADDRESSES, ADDRESS_LIST},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response};
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
