use anyhow::anyhow;
use data_encoding::HEXLOWER;
use plutus_ledger_api::{
    csl::{csl_to_pla::TryToPLA, lib as csl},
    v3::{
        address::Address,
        crypto::LedgerBytes,
        script::{MintingPolicyHash, ScriptHash},
        value::CurrencySymbol,
    },
};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct ParseCurrencySymbol(pub CurrencySymbol);

impl FromStr for ParseCurrencySymbol {
    type Err = &'static str;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(ParseCurrencySymbol(CurrencySymbol::NativeToken(
            MintingPolicyHash(ScriptHash(LedgerBytes(
                HEXLOWER.decode(&s.to_owned().into_bytes()).unwrap(),
            ))),
        )))
    }
}

#[derive(Debug, Clone)]
pub struct ParseAddress(pub Address);

impl FromStr for ParseAddress {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(ParseAddress(
            csl::Address::from_bech32(s)
                .map_err(|err| anyhow!("Couldn't parse bech32 address: {}", err))?
                .try_to_pla()
                .map_err(|err| anyhow!("Couldn't convert address: {}", err))?,
        ))
    }
}
