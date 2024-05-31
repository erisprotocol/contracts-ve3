use cosmwasm_std::{Response, StdError};
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
}
