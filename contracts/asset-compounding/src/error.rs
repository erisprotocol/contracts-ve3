use cosmwasm_std::{CheckedFromRatioError, OverflowError, Response, StdError};
use cw_asset::AssetError;
use cw_ownable::OwnershipError;
use thiserror::Error;
use ve3_shared::error::SharedError;

pub type ContractResult = Result<Response, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
  #[error("{0}")]
  Std(#[from] StdError),

  #[error("{0}")]
  OwnershipError(#[from] OwnershipError),

  #[error("{0}")]
  AssetError(#[from] AssetError),

  #[error("{0}")]
  SharedError(#[from] SharedError),

  #[error("{0}")]
  OverflowError(#[from] OverflowError),
  #[error("{0}")]
  CheckedFromRatioError(#[from] CheckedFromRatioError),

  #[error("config value too high: {0}")]
  ConfigValueTooHigh(String),

  #[error("asset not whitelisted in gauge: {0}, asset: {1}")]
  InvalidAsset(String, String),

  #[error("Only a single asset is allowed")]
  OnlySingleAssetAllowed {},

  #[error("Amount cannot be zero")]
  AmountCannotBeZero {},
}
