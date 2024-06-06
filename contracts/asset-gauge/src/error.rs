use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;
use ve3_shared::error::SharedError;

/// This enum describes contract errors
#[derive(Error, Debug)]
pub enum ContractError {
  #[error("{0}")]
  Std(#[from] StdError),

  #[error("{0}")]
  SharedError(#[from] SharedError),

  #[error("{0}")]
  OverflowError(#[from] OverflowError),

  #[error("Unauthorized")]
  Unauthorized {},

  #[error("You can't vote with zero voting power")]
  ZeroVotingPower {},

  #[error("Invalid validator address: {0}")]
  InvalidValidatorAddress(String),

  #[error("Votes contain duplicated values")]
  DuplicatedVotes {},

  #[error("There are no validators to tune")]
  TuneNoValidators {},

  #[error("Contract can't be migrated!")]
  MigrationError {},

  #[error("Cannot clear gauge that exists.")]
  CannotClearExistingGauge {},

  #[error("Gauge does not exist.")]
  GaugeDoesNotExist {},
}
