use cosmwasm_std::{DecimalRangeExceeded, OverflowError, StdError};
use cw_asset::AssetError;
use thiserror::Error;
use ve3_shared::error::SharedError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
  #[error("{0}")]
  Std(#[from] StdError),

  #[error("{0}")]
  DecimalRangeExceeded(#[from] DecimalRangeExceeded),

  #[error("{0}")]
  SharedError(#[from] SharedError),

  #[error("{0}")]
  OverflowError(#[from] OverflowError),

  #[error("{0}")]
  AssetError(#[from] AssetError),

  #[error("Only a single asset is allowed")]
  OnlySingleAssetAllowed {},

  #[error("Asset can't be the same as reward")]
  AssetInfoCannotEqualReward {},

  #[error("Asset already whitelisted")]
  AssetAlreadyWhitelisted,

  #[error("Asset not whitelisted")]
  AssetNotWhitelisted,

  #[error("Amount cannot be zero")]
  AmountCannotBeZero {},

  #[error("Invalid Distribution")]
  InvalidDistribution {},

  #[error("Yearly take rate needs to be less or equal 50%")]
  TakeRateLessOrEqual50,
}
