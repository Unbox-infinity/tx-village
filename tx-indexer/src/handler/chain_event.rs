use crate::{
    from_oura::{FromOura, OuraParseError},
    progress_tracker::ProgressTracker,
};
use anyhow::anyhow;
use itertools::Itertools;
use num_bigint::BigInt;
use oura::model as oura;
use plutus_ledger_api::v3::{
    address::Address,
    datum::{Datum, DatumHash, OutputDatum},
    redeemer::Redeemer,
    script::ScriptHash,
    transaction::{ScriptPurpose, TransactionHash, TransactionInput, TransactionOutput, TxInInfo},
    value::{CurrencySymbol, Value},
};
use serde_with::serde_as;
use std::fmt::Debug;
use std::{collections::BTreeMap, sync::atomic::Ordering};
use tracing::{event, Level};

/// Indication of when an event happened in the context of the chain.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChainEventTime {
    pub block_number: u64,
    pub block_hash: String,
    pub slot: u64,
}

/// Chain events that the indexer is configured to produce.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ChainEvent {
    /// A filtered transaction was confirmed
    TransactionEvent {
        time: ChainEventTime,
        transaction: TransactionEventRecord,
    },

    /// Rollback event occurred
    RollbackEvent { block_slot: u64, block_hash: String },

    /// Chain synchronisation progressed
    SyncProgressEvent {
        block_slot: u64,
        block_hash: String,
        percentage: f32,
    },
}

/// Details on an transaction event (excluding unnecessary information).
#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TransactionEventRecord {
    pub hash: TransactionHash,
    pub fee: u64,
    pub size: u32,

    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TxInInfo>,
    pub mint: Value,
    #[serde_as(as = "Vec<(_, _)>")]
    pub redeemers: BTreeMap<ScriptPurpose, Redeemer>,

    #[serde_as(as = "Vec<(_, _)>")]
    pub plutus_data: BTreeMap<DatumHash, Datum>,
    // TODO(chase): Which of these would be realistically be interested in?
    // pub vkey_witnesses: Option<Vec<VKeyWitnessRecord>>,
    // pub native_witnesses: Option<Vec<NativeWitnessRecord>>,
    // pub plutus_witnesses: Option<Vec<PlutusWitnessRecord>>,
    // pub plutus_redeemers: Option<Vec<PlutusRedeemerRecord>>,
}

pub fn parse_oura_event(
    ev: oura::Event,
    progress_tracker: &mut Option<ProgressTracker>,
) -> Result<Option<ChainEvent>, OuraParseError> {
    Ok(match ev.data {
        oura::EventData::Transaction(tx_rec) => {
            event!(Level::DEBUG, label="TransactionEvent", transaction_record=?tx_rec);

            Some(ChainEvent::TransactionEvent {
                time: ChainEventTime {
                    // These unwraps should not fail.
                    block_hash: ev.context.block_hash.unwrap(),
                    block_number: ev.context.block_number.unwrap(),
                    slot: ev.context.slot.unwrap(),
                },
                transaction: TransactionEventRecord::from_oura(tx_rec)?,
            })
        }
        oura::EventData::RollBack {
            block_slot,
            block_hash,
        } => {
            event!(Level::DEBUG, label="RollbackEvent", block_slot=?block_slot, block_hash=?block_hash);
            Some(ChainEvent::RollbackEvent {
                block_slot,
                block_hash,
            })
        }
        oura::EventData::Block(block_rec) => {
            event!(Level::DEBUG, label="BlockEvent", block_record=?block_rec);
            match progress_tracker {
                Some(progress_tracker) => {
                    let block_slot = block_rec.slot;
                    let block_hash = block_rec.hash;

                    let percentage = progress_tracker.get_percentage(block_slot)?;

                    let throttled_sync_progress = (percentage * 10.0) as usize;
                    let is_updated = progress_tracker
                        .sync_progress
                        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |prev_status| {
                            if prev_status < throttled_sync_progress {
                                Some(throttled_sync_progress)
                            } else {
                                None
                            }
                        })
                        .is_ok();

                    if is_updated {
                        event!(
                            Level::INFO,
                            percentage = format!("{:.1}%", percentage),
                            ?block_slot,
                            ?block_hash,
                            label = "Chain synchronization progress"
                        );
                    }

                    Some(ChainEvent::SyncProgressEvent {
                        percentage,
                        block_slot,
                        block_hash,
                    })
                }

                None => Some(ChainEvent::SyncProgressEvent {
                    percentage: 100.0,
                    block_slot: block_rec.slot,
                    block_hash: block_rec.hash,
                }),
            }
        }
        _ => panic!("absurd: Indexer filter should only allow transaction event variant."),
    })
}

impl FromOura<oura::TransactionRecord> for TransactionEventRecord {
    fn from_oura(tx: oura::TransactionRecord) -> Result<TransactionEventRecord, OuraParseError> {
        Ok(TransactionEventRecord {
            hash: TransactionHash::from_oura(tx.hash.clone())?,
            fee: tx.fee,
            redeemers: tx
                .plutus_redeemers
                .unwrap()
                .into_iter()
                .map(|redeemer_record| {
                    // TODO(szg251): parse other redeemer tags
                    let script_purpose = match &redeemer_record.purpose[..] {
                        "spend" => {
                            let inputs = tx.inputs.as_ref().unwrap();
                            let input = inputs
                                .get(redeemer_record.input_idx as usize)
                                .ok_or(OuraParseError::ParseError(anyhow!(
                                    "No input found at redeemer index {}",
                                    redeemer_record.input_idx
                                )))?
                                .clone();

                            let transaction_input = TransactionInput {
                                transaction_id: TransactionHash::from_oura(input.tx_id)?,
                                index: BigInt::from(input.index),
                            };
                            ScriptPurpose::Spending(transaction_input)
                        }
                        "mint" => {
                            let policies = tx
                                .mint
                                .as_ref()
                                .unwrap()
                                .iter()
                                .map(|mint| mint.policy.clone())
                                .sorted()
                                .dedup()
                                .collect::<Vec<_>>();
                            let policy = policies
                                .get(redeemer_record.input_idx as usize)
                                .ok_or(OuraParseError::ParseError(anyhow!(
                                    "No mint found at redeemer index {}",
                                    redeemer_record.input_idx
                                )))?
                                .clone();

                            ScriptPurpose::Minting(CurrencySymbol::from_oura(policy)?)
                        }
                        // "cert" => ScriptPurpose::Certifying (),
                        // "reward" => ScriptPurpose::Rewarding(0)
                        _ => Err(OuraParseError::ParseError(anyhow!(
                            "Cannot parse redeemer tag variant: {}",
                            redeemer_record.purpose
                        )))?,
                    };

                    Ok((
                        script_purpose,
                        Redeemer::from_oura(redeemer_record.plutus_data)?,
                    ))
                })
                .collect::<Result<_, OuraParseError>>()?,
            size: tx.size,
            // All these unwraps should succeed since we enable `include_transaction_details`
            // in the mapper config.
            inputs: tx
                .inputs
                .unwrap()
                .into_iter()
                .map(|oura::TxInputRecord { tx_id, index }| {
                    Ok(TransactionInput {
                        transaction_id: TransactionHash::from_oura(tx_id)?,
                        index: BigInt::from(index),
                    })
                })
                .collect::<Result<_, OuraParseError>>()?,
            outputs: tx
                .outputs
                .unwrap()
                .into_iter()
                .enumerate()
                .map(
                    |(
                        index,
                        oura::TxOutputRecord {
                            address,
                            amount,
                            assets,
                            datum_hash,
                            inline_datum,
                            reference_script,
                        },
                    )| {
                        let reference = TransactionInput {
                            transaction_id: TransactionHash::from_oura(tx.hash.clone())?,
                            index: index.into(),
                        };
                        let output = TransactionOutput {
                            address: Address::from_oura(address)?,
                            datum: match (datum_hash, inline_datum) {
                                (None, None) => OutputDatum::None,
                                (_, Some(datm)) => {
                                    OutputDatum::InlineDatum(Datum::from_oura(datm.plutus_data)?)
                                }
                                (Some(dh), _) => OutputDatum::DatumHash(DatumHash::from_oura(dh)?),
                            },
                            // NOTE(chase): There is currently no way to know about reference scripts with Oura.
                            reference_script: reference_script
                                .map(ScriptHash::from_oura)
                                .transpose()?,
                            value: Value::ada_value(&BigInt::from(amount))
                                + Value::from_oura(assets.unwrap_or_default())?,
                        };

                        Ok(TxInInfo { reference, output })
                    },
                )
                .collect::<Result<_, OuraParseError>>()?,
            mint: tx.mint.map_or(Ok(Value::new()), Value::from_oura)?,
            plutus_data: tx
                .plutus_data
                .unwrap_or_default()
                .into_iter()
                .map(
                    |oura::PlutusDatumRecord {
                         plutus_data,
                         datum_hash,
                     }| {
                        Ok((
                            DatumHash::from_oura(datum_hash)?,
                            Datum::from_oura(plutus_data)?,
                        ))
                    },
                )
                .collect::<Result<_, OuraParseError>>()?,
        })
    }
}
