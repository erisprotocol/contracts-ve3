use cosmwasm_std::{Addr, QuerierWrapper};
use cw_ownable::Ownership;
use cw_storage_plus::{Item, Map};

use crate::{
  constants::{at_asset_staking, at_connector, AT_ASSET_GAUGE, AT_VOTING_ESCROW, AT_ZAPPER},
  error::SharedError,
};

use super::{
  asset_gauge::AssetGauge, asset_staking::AssetStaking, connector::Connector,
  voting_escrow::VotingEscrow, zapper::Zapper,
};

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

  pub fn assert_owner_or_address_type(
    &self,
    querier: &QuerierWrapper,
    address_type: &str,
    sender: &Addr,
  ) -> Result<(), SharedError> {
    let ownership = OWNERSHIP.query(querier, self.0.clone())?;
    if let Some(owner) = ownership.owner {
      if *sender == owner {
        return Ok(());
      }
    }

    self.assert_has_access(querier, address_type, sender)
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

  pub fn is_in_list(
    &self,
    querier: &QuerierWrapper,
    address_type: &str,
    sender: &Addr,
  ) -> Result<bool, SharedError> {
    let address_list = ADDRESS_LIST.query(querier, self.0.clone(), address_type.to_string())?;
    if let Some(allowed) = address_list {
      if allowed.contains(sender) {
        return Ok(true);
      }
    }

    Ok(false)
  }

  pub fn assert_has_access(
    &self,
    querier: &QuerierWrapper,
    address_type: &str,
    sender: &Addr,
  ) -> Result<(), SharedError> {
    // check if the address_type is allowed through the address
    let address = ADDRESSES.query(querier, self.0.clone(), address_type.to_string())?;
    if let Some(allowed) = address {
      if allowed == *sender {
        return Ok(());
      }
    }

    // fallback check if the address_type is allowed through the address list
    let address_list = ADDRESS_LIST.query(querier, self.0.clone(), address_type.to_string())?;
    if let Some(allowed) = address_list {
      if allowed.contains(sender) {
        return Ok(());
      }
    }
    Err(SharedError::UnauthorizedMissingRight(address_type.to_string(), sender.to_string()))
  }
}

pub trait ConfigExt {
  fn get_address(&self, querier: &QuerierWrapper, address_type: &str) -> Result<Addr, SharedError> {
    self.global_config().get_address(querier, address_type)
  }

  fn voting_escrow(&self, querier: &QuerierWrapper) -> Result<VotingEscrow, SharedError> {
    self.global_config().get_address(querier, AT_VOTING_ESCROW).map(VotingEscrow)
  }

  fn asset_gauge(&self, querier: &QuerierWrapper) -> Result<AssetGauge, SharedError> {
    self.global_config().get_address(querier, AT_ASSET_GAUGE).map(AssetGauge)
  }

  fn zapper(&self, querier: &QuerierWrapper) -> Result<Zapper, SharedError> {
    self.global_config().get_address(querier, AT_ZAPPER).map(Zapper)
  }

  fn asset_staking(
    &self,
    querier: &QuerierWrapper,
    gauge: &str,
  ) -> Result<AssetStaking, SharedError> {
    self.global_config().get_address(querier, &at_asset_staking(gauge)).map(AssetStaking)
  }

  fn connector(&self, querier: &QuerierWrapper, gauge: &str) -> Result<Connector, SharedError> {
    self.global_config().get_address(querier, &at_connector(gauge)).map(Connector)
  }

  fn global_config(&self) -> GlobalConfig;
}

impl ConfigExt for crate::msgs_asset_staking::Config {
  fn global_config(&self) -> GlobalConfig {
    GlobalConfig(self.global_config_addr.clone())
  }
}
impl ConfigExt for crate::msgs_voting_escrow::Config {
  fn global_config(&self) -> GlobalConfig {
    GlobalConfig(self.global_config_addr.clone())
  }
}

impl ConfigExt for crate::msgs_asset_gauge::Config {
  fn global_config(&self) -> GlobalConfig {
    GlobalConfig(self.global_config_addr.clone())
  }
}

impl ConfigExt for crate::msgs_connector_alliance::Config {
  fn global_config(&self) -> GlobalConfig {
    GlobalConfig(self.global_config_addr.clone())
  }
}

impl ConfigExt for crate::msgs_bribe_manager::Config {
  fn global_config(&self) -> GlobalConfig {
    GlobalConfig(self.global_config_addr.clone())
  }
}

impl ConfigExt for crate::msgs_connector_emission::Config {
  fn global_config(&self) -> GlobalConfig {
    GlobalConfig(self.global_config_addr.clone())
  }
}

impl ConfigExt for crate::msgs_zapper::Config {
  fn global_config(&self) -> GlobalConfig {
    GlobalConfig(self.global_config_addr.clone())
  }
}

impl ConfigExt for crate::msgs_phoenix_treasury::Config {
  fn global_config(&self) -> GlobalConfig {
    GlobalConfig(self.global_config_addr.clone())
  }
}

impl ConfigExt for crate::msgs_asset_compounding::Config {
  fn global_config(&self) -> GlobalConfig {
    GlobalConfig(self.global_config_addr.clone())
  }
}
