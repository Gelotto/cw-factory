use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("MigrationExists: a migration named '{name}' is already in progress")]
    MigrationExists { name: String },

    #[error("MigrationComplete: migration '{name}' already completed")]
    MigrationComplete { name: String },

    #[error("NotAuthorized: {reason:?}")]
    NotAuthorized { reason: String },

    #[error("ValidationError: {reason:?}")]
    ValidationError { reason: String },

    #[error("MultiplyRatioError: base: {base:?}, numerator {numerator:?}, denominator: {denominator:?}")]
    MultiplyRatioError {
        base: String,
        numerator: String,
        denominator: String,
    },
}

impl From<ContractError> for StdError {
    fn from(err: ContractError) -> Self {
        StdError::generic_err(err.to_string())
    }
}
