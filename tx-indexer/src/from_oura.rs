use ::oura::model::{MintRecord, OutputAssetRecord};
use anyhow::Context;
use data_encoding::HEXLOWER;
use num_bigint::BigInt;
use plutus_ledger_api::csl::{csl_to_pla::TryToPLA, lib as csl};
use plutus_ledger_api::v3::{
    address::Address,
    crypto::LedgerBytes,
    datum::{Datum, DatumHash},
    redeemer::Redeemer,
    script::{MintingPolicyHash, ScriptHash},
    transaction::TransactionHash,
    value::{CurrencySymbol, TokenName, Value},
};
use std::fmt::Debug;

#[derive(thiserror::Error, Debug)]
pub enum OuraParseError {
    #[error(transparent)]
    ParseError(#[from] anyhow::Error),

    #[error("Unable to convert current time: {0}")]
    TimeConversionError(tx_bakery::error::Error),
}

/// Convert an Oura transaction record type to its plutus-ledger-api counterpart
pub trait FromOura<T> {
    fn from_oura(value: T) -> Result<Self, OuraParseError>
    where
        Self: Sized;
}

impl FromOura<String> for LedgerBytes {
    fn from_oura(value: String) -> Result<Self, OuraParseError> {
        Ok(LedgerBytes(
            HEXLOWER
                .decode(&value.clone().into_bytes()[..])
                .with_context(|| "Parsing LedgerBytes from Oura")?,
        ))
    }
}

impl FromOura<String> for TransactionHash {
    fn from_oura(value: String) -> Result<Self, OuraParseError> {
        Ok(TransactionHash(
            LedgerBytes::from_oura(value).with_context(|| "Parsing TransactionHash from Oura")?,
        ))
    }
}

impl FromOura<String> for DatumHash {
    fn from_oura(value: String) -> Result<Self, OuraParseError> {
        Ok(DatumHash(LedgerBytes::from_oura(value)?))
    }
}

impl FromOura<String> for ScriptHash {
    fn from_oura(value: String) -> Result<Self, OuraParseError> {
        Ok(ScriptHash(LedgerBytes::from_oura(value)?))
    }
}

impl FromOura<String> for CurrencySymbol {
    fn from_oura(value: String) -> Result<Self, OuraParseError> {
        Ok(if value.is_empty() {
            CurrencySymbol::Ada
        } else {
            CurrencySymbol::NativeToken(MintingPolicyHash(ScriptHash(LedgerBytes::from_oura(
                value,
            )?)))
        })
    }
}

impl FromOura<String> for TokenName {
    fn from_oura(value: String) -> Result<Self, OuraParseError> {
        Ok(if value.is_empty() {
            TokenName::ada()
        } else {
            TokenName(LedgerBytes::from_oura(value)?)
        })
    }
}

impl FromOura<serde_json::Value> for Datum {
    fn from_oura(value: serde_json::Value) -> Result<Self, OuraParseError> {
        let csl_plutus_data =
            csl::encode_json_value_to_plutus_datum(value, csl::PlutusDatumSchema::DetailedSchema)
                .with_context(|| "Parsing Datum from Oura")?;

        Ok(Datum(
            csl_plutus_data
                .try_to_pla()
                .with_context(|| "Parsing Datum from Oura")?,
        ))
    }
}

impl FromOura<serde_json::Value> for Redeemer {
    fn from_oura(value: serde_json::Value) -> Result<Self, OuraParseError> {
        let csl_plutus_data =
            csl::encode_json_value_to_plutus_datum(value, csl::PlutusDatumSchema::DetailedSchema)
                .with_context(|| "Parsing Redeemer from Oura")?;

        Ok(Redeemer(
            csl_plutus_data
                .try_to_pla()
                .with_context(|| "Parsing Redeemer from Oura")?,
        ))
    }
}

impl FromOura<String> for Address {
    fn from_oura(value: String) -> Result<Self, OuraParseError> {
        let csl_addr = csl::Address::from_bech32(&value)
            .or_else(|_| {
                csl::ByronAddress::from_base58(&value).map(|byron_addr| byron_addr.to_address())
            })
            .with_context(|| "Parsing Address from Oura")?;

        Ok(csl_addr
            .try_to_pla()
            .with_context(|| "Parsing Address from Oura")?)
    }
}

impl FromOura<Vec<OutputAssetRecord>> for Value {
    fn from_oura(value: Vec<OutputAssetRecord>) -> Result<Self, OuraParseError> {
        value.iter().try_fold(Value::new(), |acc, x| {
            let amt = BigInt::from(x.amount);
            Ok(acc.insert_token(
                &CurrencySymbol::from_oura(x.policy.clone())?,
                &TokenName::from_oura(x.asset.clone())?,
                &amt,
            ))
        })
    }
}

impl FromOura<Vec<MintRecord>> for Value {
    fn from_oura(value: Vec<MintRecord>) -> Result<Self, OuraParseError> {
        value.iter().try_fold(Value::new(), |acc, x| {
            let amt = BigInt::from(x.quantity);
            Ok(acc.insert_token(
                &CurrencySymbol::from_oura(x.policy.clone())?,
                &TokenName::from_oura(x.asset.clone())?,
                &amt,
            ))
        })
    }
}
