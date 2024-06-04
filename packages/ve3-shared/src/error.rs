use cosmwasm_std::StdError;
use cw_asset::AssetError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum SharedError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    AssetError(#[from] AssetError),

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

    #[error("Insufficient Balance: {0}")]
    InsufficientBalance(String),
}
