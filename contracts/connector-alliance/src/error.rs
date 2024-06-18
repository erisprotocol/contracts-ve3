use cosmwasm_std::{OverflowError, StdError};
use cw_asset::AssetError;
use thiserror::Error;
use ve3_shared::error::SharedError;

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

  #[error("Empty delegation")]
  EmptyDelegation {},

  #[error("Invalid reply id {0}")]
  InvalidReplyId(u64),

  #[error("Nothing to take")]
  NothingToTake,
}
