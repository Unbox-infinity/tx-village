//! Transaction Bakery

use crate::error::{Error, Result};
use crate::metadata::TransactionMetadata;
use crate::time::time_range_into_slots;
use crate::wallet::Wallet;
use anyhow::anyhow;
use chain_query::{ChainQuery, EraSummary, Network, ProtocolParameters};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use num_bigint::BigInt;
use plutus_ledger_api::csl::{lib as csl, pla_to_csl::TryToCSL};
use plutus_ledger_api::plutus_data::IsPlutusData;
use plutus_ledger_api::v2::{
    address::{Address, AddressWithExtraInfo, Credential},
    crypto::PaymentPubKeyHash,
    datum::{Datum, DatumHash, OutputDatum},
    redeemer::{Redeemer, RedeemerWithExtraInfo},
    script::{MintingPolicyHash, ScriptHash, ValidatorHash},
    transaction::{
        ScriptPurpose, TransactionHash, TransactionInfo, TransactionInput, TransactionOutput,
        TransactionOutputWithExtraInfo, TxInInfo,
    },
    value::{CurrencySymbol, Value},
};
use std::collections::BTreeMap;
use submitter::Submitter;
use tracing::{debug, info};
use utils::script::ScriptOrRef;

pub mod chain_query;
#[cfg(feature = "clap")]
pub mod clap;
pub mod error;
pub mod metadata;
pub mod submitter;
pub mod time;
pub mod tx_info_builder;
pub mod utils;
pub mod wallet;

/// Transaction builder
///
/// The purpose of this component is to convert a raw TransactionInfo (dough)
/// into a fully baked valid transaction.
/// TxBakery does not perform IO and won't change it's internal state once initialized.
pub struct TxBakery {
    config: csl::TransactionBuilderConfig,
    data_cost: csl::DataCost,
    cost_models: csl::Costmdls,
    system_start: DateTime<Utc>,
    era_summaries: Vec<EraSummary>,
    network_id: u8,
}

/// TransactionInfo with additional context required to build a valid transaction
#[derive(Clone, Debug)]
pub struct TxWithCtx<'a> {
    pub tx_info: &'a TransactionInfo,
    pub scripts: &'a BTreeMap<ScriptHash, ScriptOrRef>,
    pub collateral_strategy: &'a CollateralStrategy,
    pub change_strategy: &'a ChangeStrategy,
    pub metadata: Option<&'a TransactionMetadata>,
    pub ex_units_map: Option<&'a BTreeMap<(csl::RedeemerTag, csl::BigNum), csl::ExUnits>>,
}

/// Options to deal with change outputs and collateral returns
#[derive(Clone, Debug)]
pub enum ChangeStrategy {
    /// Send all change to an address
    Address(Address),
    /// Use the last output of the TransactionInfo as change output (modify it's value)
    /// Collateral returns are following the address of the last output
    LastOutput,
}

impl<'a> TxWithCtx<'a> {
    pub fn new(
        tx_info: &'a TransactionInfo,
        scripts: &'a BTreeMap<ScriptHash, ScriptOrRef>,
        collateral_strategy: &'a CollateralStrategy,
        change_strategy: &'a ChangeStrategy,
    ) -> Self {
        TxWithCtx {
            tx_info,
            scripts,
            collateral_strategy,
            change_strategy,
            metadata: None,
            ex_units_map: None,
        }
    }

    /// Attach transaction metadata to the context
    pub fn with_metadata(mut self, metadata: &'a TransactionMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Explicitly add execution units instead of running the ChainQuery evaluation
    pub fn with_ex_units(
        mut self,
        ex_units_map: &'a BTreeMap<(csl::RedeemerTag, csl::BigNum), csl::ExUnits>,
    ) -> Self {
        self.ex_units_map = Some(ex_units_map);
        self
    }
}

/// Options to deal with collateral selection
#[derive(Clone, Debug)]
pub enum CollateralStrategy {
    /// Automatically pick a suitable UTxO from the transaction inputs
    Automatic {
        min_amount: u64,
        max_utxo_count: usize,
    },
    /// Explicitly set a UTxO (doesn't have to be an input UTxO)
    Explicit {
        utxos: Vec<TxInInfo>,
        min_amount: u64,
    },
    /// No collateral (for transaction without scripts)
    None,
}

impl TxBakery {
    /// Query all the parameters required to build a transaction and store it for later use.
    /// This command will call the ChainQuery service to pull certain chain parameters
    pub async fn init(chain_query: &impl ChainQuery) -> Result<Self> {
        debug!("Initialising Transaction Bakery");
        let protocol_params = chain_query.query_protocol_params().await?;
        let system_start = chain_query.query_system_start().await?;
        let era_summaries = chain_query.query_era_summaries().await?;
        let network = chain_query.get_network();

        Self::init_with_config(&network, &protocol_params, system_start, era_summaries).await
    }

    /// Init TxBakey with the required configurations
    /// This allows to directly inject configurations, and handle them separately from the bakery
    /// (for example prefetch and cache them)
    pub async fn init_with_config(
        network: &Network,
        protocol_params: &ProtocolParameters,
        system_start: DateTime<Utc>,
        era_summaries: Vec<EraSummary>,
    ) -> Result<Self> {
        let data_cost =
            csl::DataCost::new_coins_per_byte(&protocol_params.min_utxo_deposit_coefficient);

        let linear_fee = &csl::LinearFee::new(
            &protocol_params.min_fee_coefficient,
            &protocol_params.min_fee_constant,
        );
        let config = csl::TransactionBuilderConfigBuilder::new()
            .fee_algo(linear_fee)
            .pool_deposit(&protocol_params.stake_pool_deposit)
            .key_deposit(&protocol_params.stake_credential_deposit)
            .max_value_size(protocol_params.max_value_size.ok_or(
                Error::MissingProtocolParameter("max_value_size".to_string()),
            )?)
            .max_tx_size(protocol_params.max_transaction_size.ok_or(
                Error::MissingProtocolParameter("max_transaction_size".to_string()),
            )?)
            .coins_per_utxo_byte(&protocol_params.min_utxo_deposit_coefficient)
            .ex_unit_prices(&protocol_params.script_execution_prices.clone().ok_or(
                Error::MissingProtocolParameter("script_execution_prices".to_string()),
            )?)
            .prefer_pure_change(true);

        let config = match protocol_params.min_fee_reference_scripts {
            None => config,
            Some(ref unit_interval) => config.ref_script_coins_per_byte(unit_interval),
        }
        .build()
        .unwrap();

        Ok(TxBakery {
            config,
            data_cost,
            cost_models: protocol_params
                .plutus_cost_models
                .clone()
                .ok_or(Error::MissingProtocolParameter("cost_models".to_string()))?,
            system_start,
            era_summaries,
            network_id: network.to_network_id(),
        })
    }

    /// Create a new CSL transaction builder
    fn create_tx_builder(&self) -> csl::TransactionBuilder {
        csl::TransactionBuilder::new(&self.config)
    }

    /// Convert PLA TransactionInfo inputs, redeemers and datums to a CSL transaction input builder
    fn mk_inputs(
        &self,
        inputs: &[TxInInfo],
        ref_inputs: &[TxInInfo],
        input_redeemers: &BTreeMap<TransactionInput, Redeemer>,
        input_datums: &BTreeMap<DatumHash, Datum>,
        scripts: &BTreeMap<ScriptHash, ScriptOrRef>,
        ex_units_map: Option<&BTreeMap<(csl::RedeemerTag, csl::BigNum), csl::ExUnits>>,
    ) -> Result<csl::TxInputsBuilder> {
        let mut tx_inputs_builder = csl::TxInputsBuilder::new();

        inputs
            .iter()
            .enumerate()
            .try_for_each(|(idx, TxInInfo { reference, output })| {
                let redeemer = input_redeemers.get(reference);

                match redeemer {
                    None => {
                        tx_inputs_builder
                            .add_regular_input(
                                &AddressWithExtraInfo {
                                    address: &output.address,
                                    network_tag: self.network_id,
                                }
                                .try_to_csl()?,
                                &reference.try_to_csl()?,
                                &output.value.try_to_csl()?,
                            )
                            .map_err(|err| {
                                Error::TransactionBuildError(anyhow!(
                                    "Failed to add regular input {:?}: {}",
                                    reference,
                                    err
                                ))
                            })?;
                        Ok::<(), Error>(())
                    }
                    Some(redeemer) => {
                        let script_hash = match &output.address.credential {
                            Credential::Script(ValidatorHash(script_hash)) => Ok(script_hash),
                            _ => Err(Error::Internal(anyhow!(
                                "A public key transaction input should not have a redeemer."
                            ))),
                        }?;
                        let script_or_ref = scripts
                            .get(script_hash)
                            .ok_or_else(|| Error::MissingScript(script_hash.clone()))?;

                        let csl_redeemer = RedeemerWithExtraInfo {
                            redeemer,
                            tag: &csl::RedeemerTag::new_spend(),
                            index: idx as u64,
                        }
                        .try_to_csl()?;

                        let csl_redeemer = match ex_units_map {
                            Some(ex_units_map) => {
                                Self::apply_ex_units(&csl_redeemer, ex_units_map)?
                            }
                            None => csl_redeemer,
                        };

                        let script_source = match &script_or_ref {
                            ScriptOrRef::PlutusScript(script) => {
                                csl::PlutusScriptSource::new(script)
                            }
                            ScriptOrRef::RefScript(ref_tx_in, script) => {
                                ref_inputs
                                    .iter()
                                    .any(|TxInInfo { reference, .. }| reference == ref_tx_in)
                                    .then_some(())
                                    .ok_or_else(|| {
                                        Error::MissingReferenceScript(
                                            ref_tx_in.clone(),
                                            script_or_ref.get_script_hash(),
                                        )
                                    })?;
                                csl::PlutusScriptSource::new_ref_input(
                                    &script_hash.try_to_csl()?,
                                    &ref_tx_in.try_to_csl()?,
                                    &script.language_version(),
                                    script_or_ref.get_script_size(),
                                )
                            }
                        };

                        let datum_source = match &output.datum {
                            OutputDatum::DatumHash(dh) => {
                                let Datum(input_datum) = input_datums
                                    .get(dh)
                                    .ok_or(Error::MissingDatum(dh.clone()))?;

                                Some(csl::DatumSource::new(&input_datum.try_to_csl()?))
                            }

                            OutputDatum::InlineDatum(Datum(input_datum)) => {
                                Some(match &script_or_ref {
                                    ScriptOrRef::PlutusScript(_) => {
                                        csl::DatumSource::new(&input_datum.try_to_csl()?)
                                    }

                                    ScriptOrRef::RefScript(tx_in, _) => {
                                        csl::DatumSource::new_ref_input(&tx_in.try_to_csl()?)
                                    }
                                })
                            }

                            OutputDatum::None => None,
                        };

                        let tx_input_witness = match datum_source {
                            Some(datum_source) => csl::PlutusWitness::new_with_ref(
                                &script_source,
                                &datum_source,
                                &csl_redeemer,
                            ),
                            None => csl::PlutusWitness::new_with_ref_without_datum(
                                &script_source,
                                &csl_redeemer,
                            ),
                        };

                        tx_inputs_builder.add_plutus_script_input(
                            &tx_input_witness,
                            &reference.try_to_csl()?,
                            &output.value.try_to_csl()?,
                        );

                        Ok(())
                    }
                }
            })?;

        Ok(tx_inputs_builder)
    }

    /// Add transaction outputs to the builder
    /// If the change strategy if LastOutput, then we are postponing the addition of this output
    /// to balancing
    fn mk_outputs(
        &self,
        outputs: &Vec<TransactionOutput>,
        change_strategy: &ChangeStrategy,
        scripts: &BTreeMap<ScriptHash, ScriptOrRef>,
    ) -> Result<Vec<csl::TransactionOutput>> {
        let normal_outputs = match change_strategy {
            ChangeStrategy::Address(_) => outputs.as_slice(),
            ChangeStrategy::LastOutput => &outputs[..(outputs.len() - 1)],
        };
        let scripts = &scripts
            .iter()
            .map(|(hash, script)| (hash.clone(), script.clone().get_script()))
            .collect();
        Ok(normal_outputs
            .iter()
            .map(|transaction_output| {
                TransactionOutputWithExtraInfo {
                    scripts,
                    network_id: self.network_id,
                    data_cost: &self.data_cost,
                    transaction_output,
                }
                .try_to_csl()
            })
            .collect::<std::result::Result<_, _>>()?)
    }

    /// Convert PLA mints to CSL and pair them with their corresponding redeemers
    fn mk_mints(
        tx_mint: &Value,
        ref_inputs: &[TxInInfo],
        mint_redeemers: &BTreeMap<ScriptHash, Redeemer>,
        scripts: &BTreeMap<ScriptHash, ScriptOrRef>,
        ex_units_map: Option<&BTreeMap<(csl::RedeemerTag, csl::BigNum), csl::ExUnits>>,
    ) -> Result<csl::MintBuilder> {
        let mut mint_builder = csl::MintBuilder::new();
        tx_mint
            .0
            .iter()
            .filter_map(|(cur_sym, assets)| match cur_sym {
                CurrencySymbol::NativeToken(MintingPolicyHash(script_hash)) => {
                    Some((script_hash, assets))
                }
                CurrencySymbol::Ada => None,
            })
            .enumerate()
            .try_for_each(|(idx, (script_hash, assets))| {
                assets.iter().try_for_each(|(token_name, amount)| {
                    let script_or_ref = scripts
                        .get(script_hash)
                        .ok_or(Error::MissingScript(script_hash.clone()))?;
                    let redeemer = mint_redeemers
                        .get(script_hash)
                        .ok_or(Error::MissingMintRedeemer(script_hash.clone()))?;

                    let csl_redeemer = RedeemerWithExtraInfo {
                        redeemer,
                        tag: &csl::RedeemerTag::new_mint(),
                        index: idx as u64,
                    }
                    .try_to_csl()?;

                    let csl_redeemer = match ex_units_map {
                        Some(ex_units_map) => Self::apply_ex_units(&csl_redeemer, ex_units_map)?,
                        None => csl_redeemer,
                    };

                    let script_source = match &script_or_ref {
                        ScriptOrRef::PlutusScript(script) => csl::PlutusScriptSource::new(script),
                        ScriptOrRef::RefScript(ref_tx_in, script) => {
                            ref_inputs
                                .iter()
                                .find(|TxInInfo { reference, .. }| reference == ref_tx_in)
                                .map(|_| ())
                                .ok_or_else(|| {
                                    Error::MissingReferenceScript(
                                        ref_tx_in.clone(),
                                        script_or_ref.get_script_hash(),
                                    )
                                })?;
                            csl::PlutusScriptSource::new_ref_input(
                                &script_hash.try_to_csl()?,
                                &ref_tx_in.try_to_csl()?,
                                &script.language_version(),
                                script_or_ref.get_script_size(),
                            )
                        }
                    };

                    let mint_witness =
                        csl::MintWitness::new_plutus_script(&script_source, &csl_redeemer);
                    mint_builder
                        .add_asset(
                            &mint_witness,
                            &token_name.try_to_csl()?,
                            &amount.try_to_csl()?,
                        )
                        .map_err(|err| {
                            Error::TransactionBuildError(anyhow!(
                                "Failed to add mint {:?}: {}",
                                MintingPolicyHash(script_hash.clone()),
                                err
                            ))
                        })?;
                    Ok::<(), Error>(())
                })
            })?;

        Ok(mint_builder)
    }

    /// Find suitable UTxOs to be used as a collateral. Each UTxO has to be at a pub key address,
    /// and the total Ada amount of must be at least the configured collateral amount
    // TODO(szg251): we could calculate the exact minimum collateral amount using protocol params
    fn find_collaterals(
        &self,
        min_collateral_amount: u64,
        max_utxo_count: usize,

        tx_inputs: &[TxInInfo],
    ) -> Result<Vec<TxInInfo>> {
        let min_collateral_amount = BigInt::from(min_collateral_amount);
        let (amount, collaterals) = tx_inputs
            .iter()
            .sorted_by_key(|input| -input.output.value.get_ada_amount())
            .take(max_utxo_count)
            .fold(
                (BigInt::ZERO, Vec::new()),
                |(mut acc_amount, mut collaterals), tx_in_info| {
                    if acc_amount < min_collateral_amount {
                        if let Credential::PubKey(_) = tx_in_info.output.address.credential {
                            let ada_amount = tx_in_info.output.value.get_ada_amount();
                            acc_amount += ada_amount;
                            collaterals.push(tx_in_info.clone());
                        }
                    };

                    (acc_amount, collaterals)
                },
            );

        if amount >= min_collateral_amount {
            Ok(collaterals)
        } else {
            Err(Error::NotEnoughCollaterals {
                amount,
                required: min_collateral_amount,
                utxos: collaterals,
            })
        }
    }

    fn mk_collaterals(&self, tx_inputs: &[TxInInfo]) -> Result<csl::TxInputsBuilder> {
        let mut tx_inputs_builder = csl::TxInputsBuilder::new();
        for tx_input in tx_inputs {
            let TxInInfo { reference, output } = tx_input;

            tx_inputs_builder
                .add_regular_input(
                    &output
                        .address
                        .with_extra_info(self.network_id)
                        .try_to_csl()?,
                    &reference.try_to_csl()?,
                    &output.value.try_to_csl()?,
                )
                .map_err(|err| {
                    Error::TransactionBuildError(anyhow!(
                        "Failed to add regular input {:?}: {}",
                        reference,
                        err
                    ))
                })?;
        }

        Ok(tx_inputs_builder)
    }

    /// Convert a PLA TransactionInfo into a CSL transaction builder.
    /// The result is not yet balanced and witnesses are not added. This is useful for
    /// some further manual processing of the transaction before finalising.
    pub fn mk_tx_builder(&self, tx: &TxWithCtx<'_>) -> Result<csl::TransactionBuilder> {
        let mut tx_builder = self.create_tx_builder();

        let (input_redeemers, mint_redeemers) = tx.tx_info.redeemers.0.iter().fold(
            (BTreeMap::new(), BTreeMap::new()),
            |(mut input_reds, mut mint_reds), (purpose, red)| {
                match purpose {
                    ScriptPurpose::Spending(tx_in) => {
                        input_reds.insert(tx_in.clone(), red.clone());
                    }
                    ScriptPurpose::Minting(CurrencySymbol::NativeToken(MintingPolicyHash(
                        script_hash,
                    ))) => {
                        mint_reds.insert(script_hash.clone(), red.clone());
                    }

                    _ => {}
                };
                (input_reds, mint_reds)
            },
        );

        let input_datums = tx
            .tx_info
            .datums
            .0
            .iter()
            .cloned()
            .collect::<BTreeMap<_, _>>();

        tx_builder.set_inputs(&self.mk_inputs(
            &tx.tx_info.inputs,
            &tx.tx_info.reference_inputs,
            &input_redeemers,
            &input_datums,
            tx.scripts,
            tx.ex_units_map,
        )?);

        tx.tx_info
            .reference_inputs
            .iter()
            .try_for_each(|TxInInfo { reference, output }| {
                match output.reference_script {
                    None => tx_builder.add_reference_input(&reference.try_to_csl()?),
                    Some(ref script_hash) => {
                        let script_or_ref = tx
                            .scripts
                            .get(script_hash)
                            .ok_or(Error::MissingScript(script_hash.clone()))?;
                        tx_builder.add_script_reference_input(
                            &reference.try_to_csl()?,
                            script_or_ref.get_script_size(),
                        )
                    }
                }
                Ok::<(), Error>(())
            })?;

        tx_builder.set_mint_builder(&TxBakery::mk_mints(
            &tx.tx_info.mint,
            &tx.tx_info.reference_inputs,
            &mint_redeemers,
            tx.scripts,
            tx.ex_units_map,
        )?);

        let collateral_return_address = match tx.change_strategy {
            ChangeStrategy::Address(addr) => addr,
            ChangeStrategy::LastOutput => {
                &tx.tx_info
                    .outputs
                    .last()
                    .ok_or(Error::MissingChangeOutput)?
                    .address
            }
        };

        match &tx.collateral_strategy {
            CollateralStrategy::Automatic {
                min_amount,
                max_utxo_count,
            } => {
                let tx_input =
                    self.find_collaterals(*min_amount, *max_utxo_count, &tx.tx_info.inputs)?;
                let collateral = self.mk_collaterals(&tx_input)?;
                tx_builder.set_collateral(&collateral);
                tx_builder
                    .set_total_collateral_and_return(
                        &csl::BigNum::from(*min_amount),
                        &collateral_return_address
                            .with_extra_info(self.network_id)
                            .try_to_csl()?,
                    )
                    .map_err(|source| Error::TransactionBuildError(anyhow!(source)))?;
            }
            CollateralStrategy::Explicit { utxos, min_amount } => {
                let collateral = self.mk_collaterals(utxos)?;
                tx_builder.set_collateral(&collateral);
                tx_builder
                    .set_total_collateral_and_return(
                        &csl::BigNum::from(*min_amount),
                        &collateral_return_address
                            .with_extra_info(self.network_id)
                            .try_to_csl()?,
                    )
                    .map_err(|source| Error::TransactionBuildError(anyhow!(source)))?;
            }
            CollateralStrategy::None => {}
        };

        self.mk_outputs(&tx.tx_info.outputs, tx.change_strategy, tx.scripts)?
            .iter()
            .try_for_each(|tx_out| {
                tx_builder
                    .add_output(tx_out)
                    .map_err(|source| Error::TransactionBuildError(anyhow!(source)))
            })?;

        let (validity_start, ttl) = time_range_into_slots(
            &self.era_summaries,
            &self.system_start,
            tx.tx_info.valid_range.clone(),
        )?;

        if let Some(validity_start) = validity_start {
            tx_builder.set_validity_start_interval_bignum(validity_start);
        }
        if let Some(ttl) = ttl {
            tx_builder.set_ttl_bignum(&ttl);
        }

        tx.tx_info
            .signatories
            .iter()
            .try_for_each(|PaymentPubKeyHash(pkh)| {
                tx_builder.add_required_signer(&pkh.try_to_csl()?);
                Ok::<(), Error>(())
            })?;

        if let Some(metadata) = tx.metadata {
            tx_builder.set_metadata(&metadata.try_into()?);
        }

        Ok(tx_builder)
    }

    pub fn mk_tx_body(&self, tx: &TxWithCtx<'_>) -> Result<csl::TransactionBody> {
        let tx_builder = self.mk_tx_builder(tx)?;

        let (datums, redeemers) = TxBakery::extract_witnesses(tx.tx_info)?;

        self.balance_transaction(tx, tx_builder, &datums, &redeemers)
    }

    /// Extract witness datums and redeemers from TransactionInfo
    /// Inline datums are embedded in the transaction body, so they won't appear here
    /// Redeemers execution units are set to 0
    fn extract_witnesses(
        tx_info: &TransactionInfo,
    ) -> Result<(Vec<csl::PlutusData>, Vec<csl::Redeemer>)> {
        let datums = tx_info
            .datums
            .0
            .iter()
            .map(|(_dh, Datum(d))| Ok(d.to_plutus_data().try_to_csl()?))
            .collect::<Result<_>>()?;

        let redeemers = tx_info
            .redeemers
            .0
            .iter()
            .filter_map(|(script_purpose, redeemer)| match script_purpose {
                ScriptPurpose::Spending(reference) => tx_info
                    .inputs
                    .iter()
                    .enumerate()
                    .find(|(_idx, tx_input)| &tx_input.reference == reference)
                    .map(|(idx, _)| {
                        Ok(RedeemerWithExtraInfo {
                            redeemer,
                            tag: &csl::RedeemerTag::new_spend(),
                            index: idx as u64,
                        }
                        .try_to_csl()?)
                    }),
                ScriptPurpose::Minting(reference) => tx_info
                    .mint
                    .0
                    .iter()
                    .filter(|(cur_sym, _)| match cur_sym {
                        CurrencySymbol::NativeToken(_) => true,
                        CurrencySymbol::Ada => false,
                    })
                    .enumerate()
                    .find(|(_idx, (currency_symbol, _assets))| currency_symbol == &reference)
                    .map(|(idx, _)| {
                        Ok(RedeemerWithExtraInfo {
                            redeemer,
                            tag: &csl::RedeemerTag::new_mint(),
                            index: idx as u64,
                        }
                        .try_to_csl()?)
                    }),
                _ => Some(Err(Error::Unsupported(
                    "Only spending and minting redeemers are supported".to_string(),
                ))),
            })
            .collect::<Result<_>>()?;

        Ok((datums, redeemers))
    }

    /// Collect witness scripts (validators and mintig policies)
    fn witness_scripts(
        tx_info: &TransactionInfo,
        scripts: &BTreeMap<ScriptHash, ScriptOrRef>,
    ) -> Result<Vec<csl::PlutusScript>> {
        Ok(tx_info
            .inputs
            .iter()
            .filter_map(
                |TxInInfo {
                     reference: _,
                     output,
                 }| {
                    match &output.address {
                        Address {
                            credential: Credential::Script(ValidatorHash(script_hash)),
                            staking_credential: _,
                        } => Some(script_hash),
                        _ => None,
                    }
                },
            )
            .chain(tx_info.mint.0.iter().filter_map(|(cs, _)| match cs {
                CurrencySymbol::NativeToken(MintingPolicyHash(script_hash)) => Some(script_hash),
                _ => None,
            }))
            .map(|script_hash| {
                scripts
                    .get(script_hash)
                    .cloned()
                    .ok_or(Error::MissingScript((*script_hash).clone()))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter_map(|script_or_ref| match script_or_ref {
                ScriptOrRef::PlutusScript(script) => Some(script),
                ScriptOrRef::RefScript(_, _) => None,
            })
            .collect())
    }

    /// Calculate script integrity hash
    fn calc_script_data_hash(
        &self,
        mut tx_builder: csl::TransactionBuilder,
        wit_datums: &[csl::PlutusData],
        wit_redeemers: &[csl::Redeemer],
    ) -> csl::TransactionBuilder {
        debug!("Calculating script integrity hash");
        let mut redeemers = csl::Redeemers::new();
        wit_redeemers.iter().for_each(|red| redeemers.add(red));

        if !wit_datums.is_empty() || !wit_redeemers.is_empty() {
            let datums = if wit_datums.is_empty() {
                None
            } else {
                let mut ds = csl::PlutusList::new();
                wit_datums.iter().for_each(|dat| ds.add(dat));
                Some(ds)
            };

            let mut used_langs = csl::Languages::new();
            used_langs.add(csl::Language::new_plutus_v2());

            let script_data_hash = csl::hash_script_data(
                &redeemers,
                &self.cost_models.retain_language_versions(&used_langs),
                datums,
            );

            tx_builder.set_script_data_hash(&script_data_hash);
        }
        tx_builder
    }

    /// Apply execution units to redeemers
    fn apply_ex_units(
        redeemer: &csl::Redeemer,
        ex_units_map: &BTreeMap<(csl::RedeemerTag, csl::BigNum), csl::ExUnits>,
    ) -> Result<csl::Redeemer> {
        debug!("Apply execution units.");
        let key = (redeemer.tag(), redeemer.index());
        let ex_units = ex_units_map.get(&key).ok_or(Error::MissingExUnits(key))?;
        Ok(csl::Redeemer::new(
            &redeemer.tag(),
            &redeemer.index(),
            &redeemer.data(),
            ex_units,
        ))
    }

    /// Calculate script integrity hash, balance the transaction and add change if necessary
    fn balance_transaction(
        &self,
        tx: &TxWithCtx<'_>,
        mut tx_builder: csl::TransactionBuilder,
        wit_datums: &[csl::PlutusData],
        wit_redeemers: &[csl::Redeemer],
    ) -> Result<csl::TransactionBody> {
        debug!("Balance transaction");
        let mut redeemers = csl::Redeemers::new();
        wit_redeemers.iter().for_each(|red| redeemers.add(red));

        tx_builder = self.calc_script_data_hash(tx_builder, wit_datums, wit_redeemers);

        let (change_addr, change_datum) = match tx.change_strategy {
            ChangeStrategy::Address(address) => (
                AddressWithExtraInfo {
                    address,
                    network_tag: self.network_id,
                }
                .try_to_csl()?,
                None,
            ),
            ChangeStrategy::LastOutput => {
                let last_output = tx
                    .tx_info
                    .outputs
                    .last()
                    .ok_or(Error::MissingChangeOutput)?;

                (
                    AddressWithExtraInfo {
                        address: &last_output.address,
                        network_tag: self.network_id,
                    }
                    .try_to_csl()?,
                    last_output.datum.try_to_csl()?,
                )
            }
        };

        match change_datum {
            None => tx_builder
                .add_change_if_needed(&change_addr)
                .map_err(|source| Error::TransactionBuildError(anyhow!(source)))?,
            Some(output_datum) => tx_builder
                .add_change_if_needed_with_datum(&change_addr, &output_datum)
                .map_err(|source| Error::TransactionBuildError(anyhow!(source)))?,
        };

        tx_builder
            .build()
            .map_err(|source| Error::TransactionBuildError(anyhow!(source)))
    }

    /// Convert a TransactionInfo into a valid TransactionBody and prepare all witnesses (except
    /// wallet signatures)
    /// If the transaction context does not include execution units, we use Ogmios to calculate
    /// those
    pub async fn bake_balanced_tx(
        &self,
        submitter: &impl Submitter,
        tx: TxWithCtx<'_>,
    ) -> Result<csl::FixedTransaction> {
        info!("Bake balanced transaction.");
        let (datums, redeemers) = TxBakery::extract_witnesses(tx.tx_info)?;
        let plutus_scripts = TxBakery::witness_scripts(tx.tx_info, tx.scripts)?;

        let aux_data: Result<Option<csl::AuxiliaryData>> = tx
            .metadata
            .map(|metadata| {
                let mut aux_data = csl::AuxiliaryData::new();
                aux_data.set_metadata(&metadata.try_into()?);
                Ok(aux_data)
            })
            .transpose();
        let aux_data = aux_data?;

        let (tx_builder, ex_units) = match &tx.ex_units_map {
            Some(ex_units_map) => {
                debug!("Using supplied execution units.");
                (self.mk_tx_builder(&tx)?, (*ex_units_map).clone())
            }
            None => {
                debug!("Using Ogmios to calculate execution units.");
                let tx_builder = self.mk_tx_builder(&tx)?;

                let ex_units = submitter
                    .evaluate_transaction(&tx_builder, &plutus_scripts, &redeemers)
                    .await?;

                debug!("Applying execution units to transaction.");

                (
                    self.mk_tx_builder(&tx.clone().with_ex_units(&ex_units))?,
                    ex_units,
                )
            }
        };

        let redeemers_w_ex_u = redeemers
            .iter()
            .map(|r| TxBakery::apply_ex_units(r, &ex_units))
            .collect::<Result<Vec<_>>>()?;

        let tx_body = self.balance_transaction(&tx, tx_builder, &datums, &redeemers_w_ex_u)?;

        let mut witness_set = csl::TransactionWitnessSet::new();
        let mut script_witnesses = csl::PlutusScripts::new();

        plutus_scripts
            .iter()
            .for_each(|script| script_witnesses.add(script));

        let mut redeemer_witnesses = csl::Redeemers::new();

        redeemers_w_ex_u
            .iter()
            .for_each(|redeemer| redeemer_witnesses.add(redeemer));

        witness_set.set_plutus_scripts(&script_witnesses);
        witness_set.set_redeemers(&redeemer_witnesses);

        let tx = csl::Transaction::new(&tx_body, &witness_set, aux_data);

        let fixed_tx = csl::FixedTransaction::from_bytes(tx.to_bytes())
            .map_err(|source| Error::TransactionBuildError(anyhow!(source)))?;

        Ok(fixed_tx)
    }

    /// Convert a TransactionInfo into a valid signed Transaction
    pub async fn bake_signed_tx(
        &self,
        submitter: &impl Submitter,
        wallet: &impl Wallet,
        tx: TxWithCtx<'_>,
    ) -> Result<csl::FixedTransaction> {
        let tx = self.bake_balanced_tx(submitter, tx).await?;
        debug!("Signing transaction.");
        Ok(wallet.sign_transaction(&tx))
    }

    /// Convert a TransactionInfo into a valid signed Transaction and submit it to the chain
    /// (Not waiting for confirmation)
    pub async fn bake_and_deliver(
        &self,
        submitter: &impl Submitter,
        wallet: &impl Wallet,
        tx: TxWithCtx<'_>,
    ) -> Result<TransactionHash> {
        let tx = self.bake_signed_tx(submitter, wallet, tx).await?;

        debug!("Submitting transaction.");
        Ok(submitter.submit_transaction(&tx).await?)
    }
}
