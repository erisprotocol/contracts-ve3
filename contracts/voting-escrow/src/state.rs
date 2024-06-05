use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, DepsMut, Uint128};
use cw_asset::Asset;
use cw_storage_plus::{Item, Map, SnapshotMap, Strategy};
use ve3_shared::voting_escrow::{AssetInfoConfig, Config, Extension, Trait};

use crate::{contract::Operation, error::ContractError};

/// This structure stores points along the checkpoint history for every vAMP staker.
#[cw_serde]
pub struct Point {
    /// The staker's vAMP voting power
    pub power: Uint128,
    /// The start period when the staker's voting power start to decrease
    pub start: u64,
    /// The period when the lock should expire
    pub end: u64,
    /// Weekly voting power decay
    pub slope: Uint128,

    /// fixed amount
    pub fixed: Uint128,
}

/// This structure stores data about the lockup position for a specific vAMP staker.
#[cw_serde]
pub struct Lock {
    /// The total amount of tokens that were deposited in the ve position
    pub asset: Asset,
    /// Underlying amount of tokens during creation / update of the lock
    pub underlying_amount: Uint128,
    /// The start period when the lock was created
    pub start: u64,
    /// The period when the lock position expires
    pub end: u64,
    /// the last period when the lock's time was increased
    pub last_extend_lock_period: u64,
    /// owner of the lock, always synchronized with the NFT, but tracked for history purposes
    pub owner: Addr,
}

impl Lock {
    pub fn get_nft_extension(&self) -> Extension {
        Extension {
            name: None,
            description: None,
            image: None,
            attributes: Some(vec![
                Trait {
                    display_type: None,
                    trait_type: "asset".to_string(),
                    value: self.asset.to_string(),
                },
                Trait {
                    display_type: None,
                    trait_type: "start".to_string(),
                    value: self.start.to_string(),
                },
                Trait {
                    display_type: None,
                    trait_type: "end".to_string(),
                    value: self.end.to_string(),
                },
            ]),
        }
    }

    pub fn update_underlying(
        &mut self,
        deps: &DepsMut,
        asset_config: &AssetInfoConfig,
    ) -> Result<Operation, ContractError> {
        let new_underlying_amount =
            asset_config.get_underlying_amount(&deps.querier, self.asset.amount)?;
        self.update_underlying_value(new_underlying_amount)
    }

    pub fn update_underlying_value(
        &mut self,
        new_underlying_amount: Uint128,
    ) -> Result<Operation, ContractError> {
        let add_reduce_underlying = if new_underlying_amount > self.underlying_amount {
            Operation::Add(new_underlying_amount - self.underlying_amount)
        } else if new_underlying_amount == self.underlying_amount {
            Operation::None
        } else {
            Operation::Reduce(self.underlying_amount - new_underlying_amount)
        };

        self.underlying_amount = new_underlying_amount;
        Ok(add_reduce_underlying)
    }
}

/// Stores the contract config at the given key
pub const CONFIG: Item<Config> = Item::new("config");

/// Stores all user lock history
pub const LOCKED: SnapshotMap<&str, Lock> =
    SnapshotMap::new("locked", "locked__checkpoints", "locked__changelog", Strategy::EveryBlock);

/// Stores the checkpoint history for every token (token_id => period)
/// Total voting power checkpoints are stored using a ("0" => period) key
pub const HISTORY: Map<(&str, u64), Point> = Map::new("history");

/// Scheduled slope changes per period (week)
pub const SLOPE_CHANGES: Map<u64, Uint128> = Map::new("slope_changes");

/// Last period when a scheduled slope change was applied
pub const LAST_SLOPE_CHANGE: Item<u64> = Item::new("last_slope_change");

/// Contains blacklisted staker addresses
pub const BLACKLIST: Item<Vec<Addr>> = Item::new("blacklist");

pub const TOKEN_ID: Item<Uint128> = Item::new("token_id");
