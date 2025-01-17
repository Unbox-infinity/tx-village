use super::error::OgmiosError;
use anyhow::anyhow;
use chrono::Duration;
use data_encoding::HEXLOWER;
use jsonrpsee::core::traits::ToRpcParams;
use num_bigint::BigInt;
use plutus_ledger_api as pla;
use plutus_ledger_api::csl::{csl_to_pla::TryToPLA, lib as csl, pla_to_csl::TryToCSL};
use serde::{Deserialize, Serialize};
use serde_json::value::{RawValue, Value as JsonValue};
use std::collections::BTreeMap;
use std::str::FromStr;
use tx_bakery::chain_query::{self, FullTransactionOutput};

pub type Result<T> = std::result::Result<T, OgmiosError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OgmiosHealth {
    // These fields are not parsed:
    //
    // "metrics": {
    //     "totalUnrouted": 1,
    //     "totalMessages": 30029,
    //     "runtimeStats": {
    //         "gcCpuTime": 1233009354,
    //         "cpuTime": 81064672549,
    //         "maxHeapSize": 41630,
    //         "currentHeapSize": 1014
    //     },
    //     "totalConnections": 10,
    //     "sessionDurations": {
    //         "max": 57385,
    //         "mean": 7057,
    //         "min": 0
    //     },
    //     "activeConnections": 0
    // },
    // "startTime": "2021-03-15T16:16:41.470782977Z",
    // "lastTipUpdate": "2021-03-15T16:28:36.853115034Z",
    // "lastKnownTip": {
    //     "hash": "c29428f386c701c1d1ba1fd259d4be78921ee9ee6c174eac898245ceb55e8061",
    //     "blockNo": 5034297,
    //     "slot": 15520688
    // },
    pub network_synchronization: f32,
    pub current_era: String,
    pub connection_status: String,
    pub current_epoch: i64,
    pub slot_in_epoch: i64,
    pub version: String,
    pub network: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct QueryLedgerStateUtxoByAddressParams {
    pub addresses: Vec<String>,
}

impl ToRpcParams for QueryLedgerStateUtxoByAddressParams {
    fn to_rpc_params(self) -> std::result::Result<Option<Box<RawValue>>, serde_json::Error> {
        serde_json::value::to_raw_value(&self).map(Some)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct OutputReference {
    transaction: TransactionId,
    index: u32,
}

impl TryFrom<pla::v3::transaction::TransactionInput> for OutputReference {
    type Error = OgmiosError;

    fn try_from(
        i: pla::v3::transaction::TransactionInput,
    ) -> std::result::Result<Self, Self::Error> {
        let transaction = TransactionId::from(i.transaction_id);
        let index = u32::try_from(i.index).map_err(|err| OgmiosError::ConversionError {
            label: "BigInt to u32".into(),
            source: anyhow!(err),
        })?;
        Ok(Self { transaction, index })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct QueryLedgerStateUtxoByOutputReferenceParams {
    #[serde(rename = "outputReferences")]
    pub output_references: Vec<OutputReference>,
}

impl ToRpcParams for QueryLedgerStateUtxoByOutputReferenceParams {
    fn to_rpc_params(self) -> std::result::Result<Option<Box<RawValue>>, serde_json::Error> {
        serde_json::value::to_raw_value(&self).map(Some)
    }
}

pub(crate) type QueryNetworkStartTimeResponse = String;

pub(crate) type QueryLedgerStateEraSummariesResponse = Vec<EraSummary>;

pub(crate) type QueryLedgerStateUtxoResponse = Vec<Utxo>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum QueryLedgerStateTipResponse {
    Origin(String),
    Point { slot: u64, id: String },
}

impl From<QueryLedgerStateTipResponse> for chain_query::ChainTip {
    fn from(tip: QueryLedgerStateTipResponse) -> chain_query::ChainTip {
        match tip {
            QueryLedgerStateTipResponse::Origin(_) => Self::Origin,
            QueryLedgerStateTipResponse::Point { slot, id } => Self::Point { slot, id },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct EraSummary {
    pub start: EraTime,
    pub end: Option<EraTime>,
    pub parameters: EraParams,
}

impl TryFrom<EraSummary> for chain_query::EraSummary {
    type Error = OgmiosError;

    fn try_from(era_summary: EraSummary) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            start: era_summary.start.try_into()?,
            end: era_summary.end.map(|end| end.try_into()).transpose()?,
            parameters: era_summary.parameters.into(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct EraTime {
    pub time: Seconds,
    pub slot: u64,
    pub epoch: u64,
}

impl TryFrom<EraTime> for chain_query::EraTime {
    type Error = OgmiosError;

    fn try_from(era_time: EraTime) -> std::result::Result<chain_query::EraTime, Self::Error> {
        Ok(chain_query::EraTime {
            time: Duration::try_seconds(era_time.time.seconds as i64).ok_or(
                OgmiosError::ConversionError {
                    label: "EraSummary start".to_string(),
                    source: anyhow!("Unable to convert seconds."),
                },
            )?,
            slot: era_time.slot,
            epoch: era_time.epoch,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EraParams {
    pub epoch_length: u64,
    pub slot_length: MilliSeconds,
    pub safe_zone: Option<u64>,
}

impl From<EraParams> for chain_query::EraParameters {
    fn from(era_summary: EraParams) -> chain_query::EraParameters {
        chain_query::EraParameters {
            epoch_length: era_summary.epoch_length,
            slot_length: era_summary.slot_length.milliseconds,
            safe_zone: era_summary.safe_zone,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Seconds {
    pub seconds: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MilliSeconds {
    pub milliseconds: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Utxo {
    pub transaction: TransactionId,
    pub index: u32,
    pub address: String,
    pub value: Value,
    pub datum_hash: Option<String>,
    pub datum: Option<String>,
    pub script: Option<Script>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TransactionId {
    pub id: String,
}

impl ToRpcParams for TransactionId {
    fn to_rpc_params(self) -> std::result::Result<Option<Box<RawValue>>, serde_json::Error> {
        serde_json::value::to_raw_value(&self).map(Some)
    }
}

type Value = BTreeMap<String, BTreeMap<String, u64>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum Script {
    Native {
        language: String,
        json: JsonValue,
        cbor: String,
    },
    Plutus {
        language: String,
        cbor: String,
    },
}

impl TryFrom<&Utxo> for pla::v3::transaction::TransactionInput {
    type Error = OgmiosError;

    fn try_from(resp: &Utxo) -> Result<Self> {
        Ok(Self {
            transaction_id: (&resp.transaction).try_into()?,
            index: resp.index.into(),
        })
    }
}

impl TryFrom<&TransactionId> for pla::v3::transaction::TransactionHash {
    type Error = OgmiosError;

    fn try_from(resp: &TransactionId) -> Result<Self> {
        Ok(Self(pla::v3::crypto::LedgerBytes(
            HEXLOWER
                .decode(&resp.id.clone().into_bytes())
                .map_err(|source| OgmiosError::ConversionError {
                    label: "TransactionHash".to_string(),
                    source: anyhow!(source),
                })?,
        )))
    }
}

impl TryFrom<TransactionId> for pla::v3::transaction::TransactionHash {
    type Error = OgmiosError;

    fn try_from(resp: TransactionId) -> Result<pla::v3::transaction::TransactionHash> {
        (&resp).try_into()
    }
}

impl From<&pla::v3::transaction::TransactionHash> for TransactionId {
    fn from(tx_hash: &pla::v3::transaction::TransactionHash) -> TransactionId {
        let pla::v3::transaction::TransactionHash(pla::v3::crypto::LedgerBytes(bytes)) = tx_hash;
        TransactionId {
            id: HEXLOWER.encode(bytes),
        }
    }
}

impl From<pla::v3::transaction::TransactionHash> for TransactionId {
    fn from(tx_hash: pla::v3::transaction::TransactionHash) -> TransactionId {
        From::from(&tx_hash)
    }
}

impl TryFrom<&Utxo> for FullTransactionOutput {
    type Error = OgmiosError;

    fn try_from(utxo: &Utxo) -> Result<FullTransactionOutput> {
        let value = pla::v3::value::Value(
            utxo.value
                .iter()
                .map(|(cur_sym, tokens)| {
                    let cur_sym: Result<_> = if cur_sym == "ada" {
                        Ok(pla::v3::value::CurrencySymbol::Ada)
                    } else {
                        Ok(pla::v3::value::CurrencySymbol::NativeToken(
                            pla::v3::script::MintingPolicyHash(pla::v3::script::ScriptHash(
                                pla::v3::crypto::LedgerBytes(
                                    HEXLOWER.decode(&cur_sym.clone().into_bytes()).map_err(
                                        |source| OgmiosError::ConversionError {
                                            label: "MintingPolicyHash".to_string(),
                                            source: anyhow!(source),
                                        },
                                    )?,
                                ),
                            )),
                        ))
                    };

                    let tokens: Result<_> = tokens
                        .iter()
                        .map(|(token_name, amount)| {
                            let token_name: Result<_> = if token_name == "lovelace" {
                                Ok(pla::v3::value::TokenName::ada())
                            } else {
                                Ok(pla::v3::value::TokenName(pla::v3::crypto::LedgerBytes(
                                    HEXLOWER.decode(&token_name.clone().into_bytes()).map_err(
                                        |source| OgmiosError::ConversionError {
                                            label: "TokenName".to_string(),
                                            source: anyhow!(source),
                                        },
                                    )?,
                                )))
                            };
                            Ok((token_name?, BigInt::from(*amount)))
                        })
                        .collect();

                    Ok((cur_sym?, tokens?))
                })
                .collect::<Result<BTreeMap<pla::v3::value::CurrencySymbol, _>>>()?,
        );

        let datum = if let Some(dh) = &utxo.datum_hash {
            let bytes = HEXLOWER
                .decode(&dh.clone().into_bytes())
                .map_err(|source| OgmiosError::ConversionError {
                    label: "DatumHash".to_string(),
                    source: anyhow!(source),
                })?;

            pla::v3::datum::OutputDatum::DatumHash(pla::v3::datum::DatumHash(
                pla::v3::crypto::LedgerBytes(bytes),
            ))
        } else if let Some(d) = &utxo.datum {
            let plutus_data =
                &csl::PlutusData::from_hex(d).map_err(|source| OgmiosError::ConversionError {
                    label: "PlutusData".to_string(),
                    source: anyhow!(source),
                })?;
            pla::v3::datum::OutputDatum::InlineDatum(pla::v3::datum::Datum(
                plutus_data.try_to_pla()?,
            ))
        } else {
            pla::v3::datum::OutputDatum::None
        };

        let reference_script = utxo
            .script
            .clone()
            .map(|script| -> Result<_> {
                match script {
                    Script::Native { cbor, .. } => {
                        let script = csl::NativeScript::from_hex(&cbor).map_err(|source| {
                            OgmiosError::ConversionError {
                                label: "NativeScript".to_string(),
                                source: anyhow!(source),
                            }
                        })?;

                        Ok(tx_bakery::utils::script::Script::NativeScript(script))
                    }
                    Script::Plutus { cbor, language } => {
                        let plutus_version = match &language[..] {
                            "plutus:v1" => csl::Language::new_plutus_v1(),
                            "plutus:v2" => csl::Language::new_plutus_v2(),
                            "plutus:v3" => csl::Language::new_plutus_v3(),
                            _ => Err(OgmiosError::ConversionError {
                                label: "Plutus language".to_string(),
                                source: anyhow!("Couldn't parse Plutus language version."),
                            })?,
                        };

                        let flat_bytes =
                            hex::decode(cbor).map_err(|err| OgmiosError::ConversionError {
                                label: "Plutus script".to_string(),
                                source: anyhow!(
                                    "Couldn't decode hex encoded plutus script: {}",
                                    err
                                ),
                            })?;

                        let mut serializer = cbor_event::se::Serializer::new_vec();
                        serializer.write_bytes(flat_bytes).unwrap();
                        let script_bytes = serializer.finalize();

                        let script = csl::PlutusScript::from_bytes_with_version(
                            script_bytes,
                            &plutus_version,
                        )
                        .map_err(|source| {
                            OgmiosError::ConversionError {
                                label: "PlutusScript".to_string(),
                                source: anyhow!(source),
                            }
                        })?;

                        Ok(tx_bakery::utils::script::Script::PlutusScript(script))
                    }
                }
            })
            .transpose()?;

        Ok(FullTransactionOutput {
            address: csl::Address::from_bech32(&utxo.address)
                .map_err(|source| OgmiosError::ConversionError {
                    label: "Address".to_string(),
                    source: anyhow!(source),
                })?
                .try_to_pla()?,
            value,
            datum,
            reference_script,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct EvaluateTransactionParams {
    pub transaction: TransactionCbor,
    #[serde(rename(deserialize = "additionalUtxo"))]
    pub additional_utxo: Vec<Utxo>,
}

impl ToRpcParams for EvaluateTransactionParams {
    fn to_rpc_params(self) -> std::result::Result<Option<Box<RawValue>>, serde_json::Error> {
        serde_json::value::to_raw_value(&self).map(Some)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct TransactionCbor {
    pub cbor: String,
}

pub(crate) type EvaluateTransactionResponse = Vec<Budgets>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Budgets {
    pub validator: Validator,
    pub budget: Budget,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Validator {
    pub purpose: String,
    pub index: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Budget {
    pub memory: u64,
    pub cpu: u64,
}

pub(crate) type SubmitTransactionParams = EvaluateTransactionParams;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SubmitTransactionResponse {
    pub transaction: TransactionId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AcquireMempoolResponse {
    pub acquired: String,
    pub slot: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ReleaseMempoolResponse {
    pub released: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum NextTransactionResponse {
    // We could request a full transaction
    TransactionId { transaction: Option<TransactionId> },
}

pub type QueryLedgerStateProtocolParametersResponse = ProtocolParameters;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolParameters {
    pub min_fee_coefficient: u64,
    pub min_fee_constant: AdaOnly,
    pub min_fee_reference_scripts: Option<MinFeeReferenceScripts>,
    pub min_utxo_deposit_coefficient: u64,
    pub min_utxo_deposit_constant: AdaOnly,
    pub max_block_body_size: Bytes,
    pub max_block_header_size: Bytes,
    pub max_transaction_size: Option<Bytes>,
    pub max_value_size: Option<Bytes>,
    pub extra_entropy: Option<Nonce>,
    pub stake_credential_deposit: AdaOnly,
    pub stake_pool_deposit: AdaOnly,
    pub stake_pool_retirement_epoch_bound: u64,
    pub stake_pool_pledge_influence: Ratio,
    pub min_stake_pool_cost: AdaOnly,
    pub desired_number_of_stake_pools: u64,
    pub federated_block_production_ratio: Option<Ratio>,
    pub monetary_expansion: Ratio,
    pub treasury_expansion: Ratio,
    pub collateral_percentage: Option<u64>,
    pub max_collateral_inputs: Option<u64>,
    pub plutus_cost_models: Option<CostModels>,
    pub script_execution_prices: Option<ScriptExecutionPrices>,
    pub max_execution_units_per_transaction: Option<ExecutionUnits>,
    pub max_execution_units_per_block: Option<ExecutionUnits>,
    pub max_reference_scripts_size: Bytes,
    pub stake_pool_voting_thresholds: Option<StakePoolVotingThresholds>,
    pub constitutional_committee_min_size: Option<u64>,
    pub constitutional_committee_max_term_length: Option<u64>,
    pub governance_action_lifetime: Option<Epoch>,
    pub governance_action_deposit: Option<AdaOnly>,
    pub delegate_representative_voting_thresholds: Option<DelegateRepresentativeVotingThresholds>,
    pub delegate_representative_deposit: Option<AdaOnly>,
    pub delegate_representative_max_idle_time: Option<Epoch>,
    pub version: ProtocolVersion,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bytes {
    pub bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaOnly {
    pub ada: LovelaceOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LovelaceOnly {
    pub lovelace: i64,
}

pub type Nonce = String;

#[derive(Clone, Debug)]
pub struct Ratio {
    numerator: u64,
    denominator: u64,
}

impl<'de> serde::Deserialize<'de> for Ratio {
    fn deserialize<D>(deserilizer: D) -> std::result::Result<Ratio, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let str = String::deserialize(deserilizer)?;
        let (left, right) = str
            .split_once('/')
            .ok_or(serde::de::Error::custom("Not a valid ratio"))?;
        Ok(Ratio {
            numerator: FromStr::from_str(left).map_err(|err: std::num::ParseIntError| {
                serde::de::Error::custom(format!("Ratio numerator error: {:?}", err))
            })?,
            denominator: FromStr::from_str(right).map_err(|err: std::num::ParseIntError| {
                serde::de::Error::custom(format!("Ratio denominator error: {:?}", err))
            })?,
        })
    }
}

impl Serialize for Ratio {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}/{}", self.numerator, self.denominator))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptExecutionPrices {
    pub memory: Ratio,
    pub cpu: Ratio,
}

pub type CostModels = BTreeMap<String, Vec<i64>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionUnits {
    pub memory: i64,
    pub cpu: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinFeeReferenceScripts {
    range: u32,
    base: f64,
    multiplier: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StakePoolVotingThresholds {
    pub no_confidence: Ratio,
    pub constitutional_committee: ConstitutionalCommittee,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConstitutionalCommittee {
    pub default: Ratio,
    #[serde(rename(deserialize = "stateOfNoConfidence"))]
    pub state_of_no_confidence: Ratio,
}
pub type Epoch = u64;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateRepresentativeVotingThresholds {
    pub no_confidence: Ratio,
    pub constitution: Ratio,
    pub constitutional_committee: ConstitutionalCommittee,
    pub hard_fork_initiation: Ratio,
    pub protocol_parameters_update: ProtocolParametersUpdate,
    pub treasury_withdrawals: Ratio,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolParametersUpdate {
    pub network: Ratio,
    pub economic: Ratio,
    pub technical: Ratio,
    pub governance: Ratio,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: Option<u32>,
}

impl TryFrom<ProtocolParameters> for chain_query::ProtocolParameters {
    type Error = OgmiosError;

    fn try_from(pparams: ProtocolParameters) -> Result<Self> {
        let script_execution_prices = match pparams.script_execution_prices {
            None => None,
            Some(ex_prices) => Some(csl::ExUnitPrices::new(
                &csl::UnitInterval::new(
                    &csl::BigNum::from(ex_prices.memory.numerator),
                    &csl::BigNum::from(ex_prices.memory.denominator),
                ),
                &csl::UnitInterval::new(
                    &csl::BigNum::from(ex_prices.cpu.numerator),
                    &csl::BigNum::from(ex_prices.cpu.denominator),
                ),
            )),
        };

        Ok(chain_query::ProtocolParameters {
            min_fee_coefficient: csl::BigNum::from(pparams.min_fee_coefficient),
            min_fee_constant: csl::BigNum::from(pparams.min_fee_constant.ada.lovelace as u64),
            min_fee_reference_scripts: pparams
                .min_fee_reference_scripts
                .map(|min_fee| to_unit_interval(min_fee.base))
                .transpose()?,
            min_utxo_deposit_coefficient: csl::BigNum::from(pparams.min_utxo_deposit_coefficient),
            min_utxo_deposit_constant: csl::BigNum::from(
                pparams.min_utxo_deposit_constant.ada.lovelace as u64,
            ),
            stake_pool_deposit: csl::BigNum::from(pparams.stake_pool_deposit.ada.lovelace as u64),
            stake_credential_deposit: csl::BigNum::from(
                pparams.stake_credential_deposit.ada.lovelace as u64,
            ),
            max_value_size: pparams.max_value_size.map(|x| x.bytes as u32),

            max_transaction_size: pparams.max_transaction_size.map(|x| x.bytes as u32),
            script_execution_prices,
            plutus_cost_models: pparams.plutus_cost_models.map(to_costmdls),
        })
    }
}

fn to_unit_interval(float: f64) -> Result<csl::UnitInterval> {
    num::rational::Ratio::from_float(float)
        .ok_or(OgmiosError::ConversionError {
            label: "UnitInterval".to_string(),
            source: anyhow!("Couldn't convert floating number to ratio.",),
        })
        .and_then(|ratio| {
            Ok(csl::UnitInterval::new(
                &csl::BigNum::from(<u64>::try_from(ratio.numer()).map_err(|err| {
                    OgmiosError::ConversionError {
                        label: "UnitInterval".to_string(),
                        source: anyhow!(err),
                    }
                })?),
                &csl::BigNum::from(<u64>::try_from(ratio.denom()).map_err(|err| {
                    OgmiosError::ConversionError {
                        label: "UnitInterval".to_string(),
                        source: anyhow!(err),
                    }
                })?),
            ))
        })
}

fn to_costmdls(cost_models: CostModels) -> csl::Costmdls {
    let mut costmdls = csl::Costmdls::new();

    cost_models.iter().for_each(|(lang, costs)| {
        let mut cost_model = csl::CostModel::new();
        costs.iter().enumerate().for_each(|(index, model)| {
            let _ = cost_model.set(index, &model.try_to_csl().unwrap());
        });
        let language = match &lang[..] {
            "plutus:v1" => csl::Language::new_plutus_v1(),
            "plutus:v2" => csl::Language::new_plutus_v2(),
            "plutus:v3" => csl::Language::new_plutus_v3(),
            _ => panic!("Unknown Plutus language version"),
        };
        costmdls.insert(&language, &cost_model);
    });
    costmdls
}

pub fn to_redeemer_tag(str: &str) -> Result<csl::RedeemerTag> {
    match str {
        "spend" => Ok(csl::RedeemerTag::new_spend()),
        "certificate" => Ok(csl::RedeemerTag::new_cert()),
        "mint" => Ok(csl::RedeemerTag::new_mint()),
        "withdrawal" => Ok(csl::RedeemerTag::new_reward()),
        _ => Err(OgmiosError::ConversionError {
            label: "RedeemerTag".to_string(),
            source: anyhow!("Invalid RedeemerTag"),
        }),
    }
}
