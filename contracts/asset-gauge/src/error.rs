use cosmwasm_std::{OverflowError, StdError};
use cw_asset::AssetError;
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

  #[error("{0}")]
  AssetError(#[from] AssetError),

  #[error("Unauthorized")]
  Unauthorized {},

  #[error("User '{0}' has no voting power in period {1}")]
  ZeroVotingPower(String, u64),

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

  #[error("Period {0} not yet finished.")]
  PeriodNotFinished(u64),

  #[error("Gauge distribution not yet executed. gauge: {0}, period {1}")]
  GaugeDistributionNotExecuted(String, u64),
}
