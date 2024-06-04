use cosmwasm_std::{Addr, QuerierWrapper};
use cw_ownable::Ownership;
use cw_storage_plus::{Item, Map};
use ve3_shared::error::SharedError;

pub struct GlobalConfig(pub Addr);

pub const OWNERSHIP: Item<Ownership<Addr>> = Item::new("ownership");
pub const ADDRESSES: Map<String, Addr> = Map::new("addresses");
pub const ADDRESS_LIST: Map<String, Vec<Addr>> = Map::new("address_list");

impl GlobalConfig {
    pub fn assert_owner(&self, querier: &QuerierWrapper, sender: &Addr) -> Result<(), SharedError> {
        let ownership = OWNERSHIP.query(querier, self.0.clone())?;

        match ownership.owner {
            Some(owner) => {
                if *sender == owner {
                    Ok(())
                } else {
                    Err(SharedError::Unauthorized {})
                }
            },
            None => Err(SharedError::Unauthorized {}),
        }
    }

    pub fn get_address(
        &self,
        querier: &QuerierWrapper,
        address_type: &str,
    ) -> Result<Addr, SharedError> {
        let address = ADDRESSES.query(querier, self.0.clone(), address_type.to_string())?;

        match address {
            Some(addr) => Ok(addr),
            None => Err(SharedError::NotFound(format!("Address Type {0}", address_type))),
        }
    }

    pub fn assert_has_access(
        &self,
        querier: &QuerierWrapper,
        address_type: &str,
        sender: &Addr,
    ) -> Result<(), SharedError> {
        // check if the address_type is allowed through the address
        let address = ADDRESSES.query(querier, self.0.clone(), address_type.to_string())?;
        match address {
            Some(allowed) => {
                if allowed == *sender {
                    return Ok(());
                }
            },
            _ => {},
        }

        // fallback check if the address_type is allowed through the address list
        let address_list = ADDRESS_LIST.query(querier, self.0.clone(), address_type.to_string())?;
        match address_list {
            Some(allowed) => {
                if allowed.contains(sender) {
                    return Ok(());
                }
            },
            _ => {},
        }
        return Err(SharedError::UnauthorizedMissingRight(
            address_type.to_string(),
            sender.to_string(),
        ));
    }
}

pub trait ConfigExt {
    fn get_address(
        &self,
        querier: &QuerierWrapper,
        address_type: &str,
    ) -> Result<Addr, SharedError>;

    fn global_config(&self) -> GlobalConfig;
}

impl ConfigExt for ve3_shared::contract_asset_staking::Config {
    fn get_address(
        &self,
        querier: &QuerierWrapper,
        address_type: &str,
    ) -> Result<Addr, SharedError> {
        GlobalConfig(self.global_config_addr.clone()).get_address(querier, address_type)
    }

    fn global_config(&self) -> GlobalConfig {
        GlobalConfig(self.global_config_addr.clone())
    }
}

impl ConfigExt for ve3_shared::voting_escrow::Config {
    fn get_address(
        &self,
        querier: &QuerierWrapper,
        address_type: &str,
    ) -> Result<Addr, SharedError> {
        GlobalConfig(self.global_config_addr.clone()).get_address(querier, address_type)
    }

    fn global_config(&self) -> GlobalConfig {
        GlobalConfig(self.global_config_addr.clone())
    }
}
