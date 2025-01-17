//! Trait for a component capable of submitting transactions

use plutus_ledger_api::{csl::lib as csl, v3::transaction::TransactionHash};
use std::collections::BTreeMap;
use std::future::Future;
use thiserror::Error;

/// Component which can submit write actions to the chain
pub trait Submitter {
    fn evaluate_transaction(
        &self,
        tx_builder: &csl::TransactionBuilder,
        plutus_scripts: &[csl::PlutusScript],
        redeemers: &[csl::Redeemer],
    ) -> impl Future<
        Output = Result<BTreeMap<(csl::RedeemerTag, csl::BigNum), csl::ExUnits>, SubmitterError>,
    >;

    /// Submit a fully build and balanced tranasaction
    fn submit_transaction(
        &self,
        tx: &csl::FixedTransaction,
    ) -> impl Future<Output = Result<TransactionHash, SubmitterError>>;

    /// Wait for transaction confirmation on the chain
    fn await_tx_confirm(
        &self,
        tx_hash: &TransactionHash,
    ) -> impl Future<Output = Result<(), SubmitterError>>;
}

#[derive(Error, Debug)]
#[error(transparent)]
pub struct SubmitterError(pub anyhow::Error);
