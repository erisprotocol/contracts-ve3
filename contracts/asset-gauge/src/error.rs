use cosmwasm_std::{
  ConversionOverflowError, Decimal256RangeExceeded, DecimalRangeExceeded, OverflowError, StdError,
};
use cw_asset::AssetError;
use thiserror::Error;
use ve3_shared::error::SharedError;

/// This enum describes contract errors
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
  #[error("{0}")]
  Std(#[from] StdError),

  #[error("{0}")]
  SharedError(#[from] SharedError),

  #[error("{0}")]
  OverflowError(#[from] OverflowError),

  #[error("{0}")]
  AssetError(#[from] AssetError),

  #[error("{0}")]
  DecimalRangeExceeded(#[from] DecimalRangeExceeded),

  #[error("{0}")]
  Decimal256RangeExceeded(#[from] Decimal256RangeExceeded),

  #[error("{0}")]
  ConversionOverflowError(#[from] ConversionOverflowError),

  #[error("User '{0}' has no voting power in period {1}")]
  ZeroVotingPower(String, u64),

  #[error("Invalid asset address: {0}")]
  InvalidAsset(String),

  #[error("Votes contain duplicated values")]
  DuplicatedVotes {},

  #[error("Cannot clear gauge that exists.")]
  CannotClearExistingGauge {},

  #[error("Period {0} not yet finished.")]
  PeriodNotFinished(u64),

  #[error("Gauge distribution not yet executed. gauge: {0}, period {1}")]
  GaugeDistributionNotExecuted(String, u64),

  #[error("Rebases can only be claimed to a permanent lock.")]
  RebaseClaimingOnlyForPermanent,

  #[error("Cannot claim rebase to target lock as assets differentiate.")]
  RebaseWrongTargetLockAsset,
}
