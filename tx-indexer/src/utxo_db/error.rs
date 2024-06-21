use thiserror::Error;
use tx_indexer::{
    database::plutus::DBTypeConversionError,
    error::{ErrorPolicy, ErrorPolicyProvider},
};

#[derive(Error, Debug)]
pub enum UtxoIndexerError {
    #[error(transparent)]
    DbError(#[from] sqlx::error::Error),

    #[error(transparent)]
    DBTypeConversionError(#[from] DBTypeConversionError),
}

impl ErrorPolicyProvider for UtxoIndexerError {
    fn get_error_policy(&self) -> ErrorPolicy<Self> {
        ErrorPolicy::Skip
    }
}