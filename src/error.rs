//! Contract error types.
//!
//! ## Error Categories
//!
//! - **Std**: Wrapper for standard CosmWasm errors
//! - **MigrationExists/MigrationComplete**: Migration session state errors
//! - **NotAuthorized**: Permission/authorization failures
//! - **ValidationError**: Input validation failures
//! - **MultiplyRatioError**: Arithmetic overflow/underflow in checked math

use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Debug, Error,)]

pub enum ContractError {
    /// Standard CosmWasm error (storage, parsing, etc.)
    #[error("{0}")]
    Std(#[from] StdError,),

    /// Attempted to create a migration session with a name that already exists
    #[error("MigrationExists: a migration named '{name}' is already in progress")]
    MigrationExists { name: String, },

    /// Attempted to step/retry a migration session that has already completed
    #[error("MigrationComplete: migration '{name}' already completed")]
    MigrationComplete { name: String, },

    /// Authorization check failed (not manager, not contract, etc.)
    #[error("NotAuthorized: {reason:?}")]
    NotAuthorized { reason: String, },

    /// Input validation failed (invalid code_id, missing config, etc.)
    #[error("ValidationError: {reason:?}")]
    ValidationError { reason: String, },

    /// Checked arithmetic operation failed (overflow/underflow)
    #[error("MultiplyRatioError: base: {base:?}, numerator {numerator:?}, denominator: {denominator:?}")]
    MultiplyRatioError {
        base: String,
        numerator: String,
        denominator: String,
    },
}

impl From<ContractError,> for StdError {
    fn from(err: ContractError,) -> Self {

        StdError::generic_err(err.to_string(),)
    }
}
