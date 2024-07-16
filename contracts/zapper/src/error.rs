use cosmwasm_std::{Response, StdError, Uint128};
use cw_asset::{AssetError, AssetInfo};
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
  SharedError(#[from] SharedError),

  #[error("{0}")]
  AssetError(#[from] AssetError),

  #[error("AssertionFailed: balance {balance} smaller than expected {expected}")]
  AssertionFailed {
    balance: Uint128,
    expected: Uint128,
  },

  #[error("No route found: from: {from} to: {to}")]
  NoRouteFound {
    from: AssetInfo,
    to: String,
  },
}
