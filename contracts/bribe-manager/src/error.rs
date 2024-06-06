use cosmwasm_std::{DivideByZeroError, OverflowError, Response, StdError};
use cw_asset::AssetError;
use thiserror::Error;
use ve3_shared::error::SharedError;

pub type ContractResult = Result<Response, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
  #[error("{0}")]
  Std(#[from] StdError),

  #[error("{0}")]
  SharedError(#[from] SharedError),

  #[error("{0}")]
  AssetError(#[from] AssetError),

  #[error("{0}")]
  OverflowError(#[from] OverflowError),
  #[error("{0}")]
  DivideByZeroError(#[from] DivideByZeroError),

  #[error("Asset not whitelisted")]
  AssetNotWhitelisted {},

  #[error("Bribes are already being distributed.")]
  BribesAlreadyDistributing {},

  #[error("No bribes to withdraw.")]
  NoBribes,

  #[error("Bribe distribution: {0}")]
  BribeDistribution(String),

  #[error("Fee can only be native.")]
  FeeCanOnlyBeNative {},
}
