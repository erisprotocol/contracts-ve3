use cosmwasm_std::{
  CheckedMultiplyRatioError, DivideByZeroError, OverflowError, Response, StdError,
};
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

  #[error("{0} {1}")]
  SharedErrorExtended(SharedError, String),

  #[error("{0}")]
  AssetError(#[from] AssetError),

  #[error("{0}")]
  OverflowError(#[from] OverflowError),

  #[error("{0}")]
  DivideByZeroError(#[from] DivideByZeroError),

  #[error("{0}")]
  CheckedMultiplyRatioError(#[from] CheckedMultiplyRatioError),

  #[error("Asset not whitelisted")]
  AssetNotWhitelisted {},

  #[error("Bribes are already being distributed.")]
  BribesAlreadyDistributing {},

  #[error("No bribes to withdraw.")]
  NoBribes,

  #[error("Bribe distribution: {0}")]
  BribeDistribution(String),

  #[error("Bribe already claimed for period {0}")]
  BribeAlreadyClaimed(u64),

  #[error("No valid periods for claiming provided")]
  NoPeriodsValid {},
}
