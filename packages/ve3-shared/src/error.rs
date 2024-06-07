use cosmwasm_std::{CheckedMultiplyRatioError, OverflowError, StdError};
use cw_asset::AssetError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum SharedError {
  #[error("{0}")]
  Std(#[from] StdError),

  #[error("{0}")]
  AssetError(#[from] AssetError),

  #[error("{0}")]
  OverflowError(#[from] OverflowError),

  #[error("{0}")]
  CheckedMultiplyRatioError(#[from] CheckedMultiplyRatioError),

  #[error("Unauthorized")]
  Unauthorized {},

  #[error("Unauthorized missing right: ({0}, {1})")]
  UnauthorizedMissingRight(String, String),

  #[error("Callbacks can only be invoked by the contract itself")]
  UnauthorizedCallbackOnlyCallableByContract {},

  #[error("Not allowed to send funds with the execution.")]
  NoFundsAllowed {},

  #[error("Not found: {0}")]
  NotFound(String),

  #[error("Not supported: {0}")]
  NotSupported(String),

  #[error("Insufficient balance: {0}")]
  InsufficientBalance(String),

  #[error("Wrong deposit: {0}")]
  WrongDeposit(String),

  #[error("Contract_name does not match: prev: {0}, new: {1}")]
  ContractMismatch(String, String),
}
