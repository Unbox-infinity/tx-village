use num_bigint::BigInt;
use plutus_ledger_api::{
    csl::{csl_to_pla::TryFromCSLError, lib as csl, pla_to_csl::TryFromPLAError},
    v3::{
        datum::DatumHash,
        script::ScriptHash,
        transaction::{TransactionInput, TxInInfo},
    },
};
use thiserror::Error;

use crate::{chain_query::ChainQueryError, submitter::SubmitterError, wallet::WalletError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unable to find redeemer for minting policy (hash: {0:?})")]
    MissingMintRedeemer(ScriptHash),

    #[error("Unable to find redeemer for datum (hash: {0:?})")]
    MissingDatum(DatumHash),

    #[error("Unable to find Plutus script in TxWithCtx (hash: {0:?})")]
    MissingScript(ScriptHash),

    #[error("Reference input containing script {0:?} is missing from the TransactionInfo reference input list.")]
    MissingReferenceScript(TransactionInput, ScriptHash),

    #[error(
        "Couldn't find enough collaterals.
         The total amount is {amount} while {required} was expected. Utxos: {utxos:?}"
    )]
    NotEnoughCollaterals {
        amount: BigInt,
        required: BigInt,
        utxos: Vec<TxInInfo>,
    },

    #[error("Change strategy is set to LastOutput, but it is missing from the TransactionInfo.")]
    MissingChangeOutput,

    #[error("Execution units was not calculated for {0:?}.")]
    MissingExUnits((csl::RedeemerTag, csl::BigNum)),

    #[error("Protocol parameter {0} is missing")]
    MissingProtocolParameter(String),

    #[error(transparent)]
    TryFromPLAError(#[from] TryFromPLAError),

    #[error(transparent)]
    TryFromCSLError(#[from] TryFromCSLError),

    #[error(transparent)]
    ConversionError(anyhow::Error),

    #[error("Unsupported {0}.")]
    Unsupported(String),

    #[error("Internal error: {0}")]
    Internal(anyhow::Error),

    #[error("Transaction building failed: {0}")]
    TransactionBuildError(anyhow::Error),

    #[error("Chain query failed: {0}")]
    ChainQueryError(#[from] ChainQueryError),

    #[error("Chain query failed: {0}")]
    WalletError(#[from] WalletError),

    #[error("Chain query failed: {0}")]
    SubmitterError(#[from] SubmitterError),

    #[error("Error occurred due to a configuration for {0}")]
    InvalidConfiguration(String),

    #[error("A POSIX time value is invalid: {0}")]
    InvalidPOSIXTime(String),
}
