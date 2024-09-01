use cosmwasm_std::{OverflowError, StdError, Uint128};
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

  #[error("Not enough balance: balance: {0}, required: {1}")]
  NotEnoughBalance(Uint128, Asset),
  #[error("Cannot claim: {0}")]
  CannotClaim(String),
  #[error("Cannot execute: {0}")]
  CannotExecute(String),
}
