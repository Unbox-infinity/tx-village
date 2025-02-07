//! Simple wallet reading the signing key(s) from disk

use std::io::Cursor;
use std::path::Path;

use anyhow::anyhow;
use data_encoding::HEXLOWER;
use futures::future::OptionFuture;
use plutus_ledger_api::csl::{csl_to_pla::ToPLA, lib as csl};
use plutus_ledger_api::v3::{
    address::{Address, Credential, StakingCredential},
    crypto::Ed25519PubKeyHash,
};
use thiserror::Error;
use tokio;
use tokio::fs;

use crate::wallet::{Wallet, WalletError};

#[derive(Error, Debug)]
pub enum KeyWalletError {
    #[error("Failed to read private key: {0}")]
    PrivateKeyReadError(std::io::Error),

    #[error("Failed to parse private key: {0}")]
    PrivateKeyParseError(anyhow::Error),
}

impl From<KeyWalletError> for WalletError {
    fn from(err: KeyWalletError) -> WalletError {
        WalletError(anyhow!(err))
    }
}

#[derive(Debug, serde::Deserialize)]
struct TextEnvelope {
    // TODO: Verify that the TextEnvelope is correct (PaymentSigningKeyShelley_ed25519 or StakeSigningKeyShelley_ed25519)
    // #[serde(rename(deserialize = "type"))]
    // data_type: String,
    // description: String,
    #[serde(rename(deserialize = "cborHex"))]
    cbor_hex: String,
}

/// Simple wallet reading the signing key(s) from disk
pub struct KeyWallet {
    pay_priv_key: csl::PrivateKey,
    pay_pkh: Ed25519PubKeyHash,
    // TODO: Use these to implement staking features
    // stk_priv_key: Option<PrivateKey>,
    // stk_pkh: Option<Ed25519PubKeyHash>,
    address: Address,
}

impl KeyWallet {
    /// Initialise a base wallet by reading the signinig keys into memory
    pub async fn new(
        payment_skey: impl AsRef<Path>,
        staking_skey: Option<impl AsRef<Path>>,
    ) -> Result<KeyWallet, KeyWalletError> {
        let pay_priv_key = Self::read_priv_key(payment_skey).await?;
        let pay_pkh: Ed25519PubKeyHash = pay_priv_key.to_public().hash().to_pla();

        let stk_priv_key = OptionFuture::from(staking_skey.map(Self::read_priv_key))
            .await
            .transpose()?;
        let stk_pkh = stk_priv_key
            .as_ref()
            .map(|priv_key| priv_key.to_public().hash().to_pla());

        let address = Address {
            credential: Credential::PubKey(pay_pkh.clone()),
            staking_credential: stk_pkh
                .clone()
                .map(|pkh| StakingCredential::Hash(Credential::PubKey(pkh))),
        };

        Ok(KeyWallet {
            pay_priv_key,
            pay_pkh,
            // stk_priv_key,
            // stk_pkh,
            address,
        })
    }

    /// Initialise an enterprise wallet by reading the signinig key into memory
    pub async fn new_enterprise(
        payment_skey: impl AsRef<Path>,
    ) -> Result<KeyWallet, KeyWalletError> {
        Self::new(payment_skey, None::<&str>).await
    }

    /// Get the private key
    async fn read_priv_key(filepath: impl AsRef<Path>) -> Result<csl::PrivateKey, KeyWalletError> {
        let skey_str = fs::read_to_string(&filepath)
            .await
            .map_err(KeyWalletError::PrivateKeyReadError)?;

        let text_envelope: TextEnvelope = serde_json::from_str(&skey_str)
            .map_err(|err| KeyWalletError::PrivateKeyParseError(anyhow!(err)))?;

        let mut raw = cbor_event::de::Deserializer::from(Cursor::new(
            HEXLOWER
                .decode(&text_envelope.cbor_hex.clone().into_bytes())
                .unwrap(),
        ));
        let bytes: Vec<u8> = raw.bytes().unwrap();

        csl::PrivateKey::from_normal_bytes(&bytes)
            .map_err(|err| KeyWalletError::PrivateKeyParseError(anyhow!(err)))
    }
}

impl Wallet for KeyWallet {
    fn sign_transaction(&self, tx: &csl::FixedTransaction) -> csl::FixedTransaction {
        let tx_hash = tx.transaction_hash();

        let witness = &csl::make_vkey_witness(&tx_hash, &self.pay_priv_key);

        let mut tx = tx.clone();
        tx.add_vkey_witness(witness);

        tx
    }

    fn get_change_pkh(&self) -> Ed25519PubKeyHash {
        self.pay_pkh.clone()
    }

    fn get_change_addr(&self) -> Address {
        self.address.clone()
    }
}
