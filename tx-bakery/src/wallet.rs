//! Wallet trait

use plutus_ledger_api::{
    csl::lib as csl,
    v3::{address::Address, crypto::Ed25519PubKeyHash},
};
use thiserror::Error;

/// Cardano wallet that has access to a private key (directly or indirectly)
/// and able to sign a transaction
pub trait Wallet {
    /// Signs a fully built transaction
    fn sign_transaction(&self, tx: &csl::FixedTransaction) -> csl::FixedTransaction;

    /// Query the public key hash used by this wallet
    fn get_change_pkh(&self) -> Ed25519PubKeyHash;

    /// Query the wallet address
    fn get_change_addr(&self) -> Address;
}

#[derive(Error, Debug)]
#[error(transparent)]
pub struct WalletError(pub anyhow::Error);
