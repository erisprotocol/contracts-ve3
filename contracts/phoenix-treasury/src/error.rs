use cosmwasm_std::{Decimal, OverflowError, StdError, Uint128};
use cw_asset::{Asset, AssetError, AssetInfo};
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

  #[error("Action cancelled: {0}")]
  ActionCancelled(u64),

  #[error("Action done: {0}")]
  ActionDone(u64),

  #[error("Sender is not allowed to veto {0}")]
  NotVetoer(String),

  #[error("Cannot veto this prop due to low spend amount: veto limit: {0}, spend amount: {1}")]
  ActionValueNotEnough(Uint128, Uint128),

  #[error("Oracle did not return any amount: {0}")]
  OracleReturnedZeroUsd(Asset),

  #[error("Expected reservation for asset info: {0}")]
  ExpectedAssetReservation(AssetInfo),

  #[error("Not enough funds: balance: {0}, required: {1}")]
  NotEnoughFunds(Uint128, Asset),

  #[error("Cannot claim: no open payment for sender")]
  CannotClaimNoOpenPayment,

  #[error("Cannot claim: vesting not yet active")]
  CannotClaimVestingNotActive,

  #[error("Cannot claim: not allowed")]
  CannotClaimNotAllowed,

  #[error("Cannot claim: nothing to claim")]
  CannotClaimNothingToClaim,

  #[error("Cannot execute: not active")]
  CannotExecuteNotActive,

  #[error("Cannot execute: Only OTC")]
  CannotExecuteOnlyOtc,

  #[error("Cannot execute: Only DCA")]
  CannotExecuteOnlyDca,

  #[error("Cannot execute: Only Milestones")]
  CannotExecuteOnlyMilestone,

  #[error("Cannot execute: Missing funds")]
  CannotExecuteMissingFunds,

  #[error("Cannot execute: DCA not active")]
  CannotExecuteDcaNotActive,

  #[error("Action not reserving any funds")]
  ActionNotReservingAnyFunds,

  #[error("Action not allowed")]
  ActionNotAllowed,

  #[error("Milestone already claimed")]
  MilestoneClaimed,

  #[error("No active, unclaimed milestone not found")]
  MilestoneNotFound,

  #[error("Missing oracle: {0}")]
  MissingOracle(cw_asset::AssetInfoBase<cosmwasm_std::Addr>),

  #[error("Otc amount bigger than available: returning: {0}, max: {1}")]
  OtcAmountBiggerThanAvailable(Uint128, Uint128),

  #[error("Otc discount too high: max {0}")]
  OtcDiscountTooHigh(Decimal),

  #[error("Dca wait for cooldown to end: {0}")]
  DcaWaitForCooldown(u64),

  #[error("Swap assets cannot be the same")]
  SwapAssetsSame,

  #[error("Cannot use VT in setup")]
  CannotUseVt,

  #[error("Cannot interact with contract. Clawback triggered.")]
  ClawbackTriggered,
}
