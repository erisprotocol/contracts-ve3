use cosmwasm_std::{OverflowError, StdError};
use cw_asset::AssetError;
use thiserror::Error;
use ve3_shared::error::SharedError;

/// This enum describes vAMP contract errors
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
  #[error("{0}")]
  Std(#[from] StdError),

  #[error("{0}")]
  SharedError(#[from] SharedError),

  #[error("{0}")]
  Overflow(#[from] OverflowError),

  #[error("{0}")]
  AssetError(#[from] AssetError),

  #[error("{0}")]
  NftError(#[from] cw721_base::ContractError),

  #[error("Asset not supported: {0}")]
  WrongAsset(String),

  #[error("No assets sent")]
  NoAssetsSent,

  #[error("Asset not supported: {0} expected: {1}")]
  WrongAssetExpected(String, String),

  #[error("You need to provide assets to create or deposit for a lock.")]
  LockRequiresAmount,

  #[error("Lock does not exist: {0}")]
  LockDoesNotExist(String),

  #[error("Lock time must be within limits (week <= lock time < 2 years)")]
  LockTimeLimitsError {},

  #[error("Lock period must be 1 or more weeks")]
  LockPeriodsError {},

  #[error("Lock is permanent: {0}")]
  LockIsPermanent(String),

  #[error("Lock is not permanent")]
  LockIsNotPermanent,

  #[error("Locks decommissioned, cannot extend or create new ones.")]
  DecommissionedError {},

  #[error("The lock time has not yet expired")]
  LockHasNotExpired {},

  #[error("The {0} address is blacklisted")]
  AddressBlacklisted(String),

  #[error("The {0} address is not blacklisted")]
  AddressNotBlacklisted(String),

  #[error("Do not send the address {0} multiple times. (Blacklist)")]
  AddressBlacklistDuplicated(String),

  #[error("Append and remove arrays are empty")]
  AddressBlacklistEmpty {},

  #[error("Checkpoint initialization error")]
  CheckpointInitializationFailed {},

  #[error("Lock has not enough funds")]
  LockNotEnoughFunds {},

  #[error("Locks need to have the same assets, tokens: {0}, {1}")]
  LocksNeedSameAssets(String, String),

  #[error("Locks need to have the same end, tokens: {0}, {1}")]
  LocksNeedSameEnd(String, String),
}
