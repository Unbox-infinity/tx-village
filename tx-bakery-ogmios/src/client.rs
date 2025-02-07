use std::collections::BTreeMap;
use std::time;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use derive_builder::Builder;
use jsonrpsee::core::traits::ToRpcParams;
use jsonrpsee::{
    core::client::ClientT,
    core::params::ObjectParams,
    rpc_params,
    ws_client::{WsClient, WsClientBuilder},
};
use plutus_ledger_api::csl::pla_to_csl::TryToCSL;
use plutus_ledger_api::{
    csl::lib as csl,
    v3::{
        address::Address,
        transaction::{TransactionHash, TransactionInput},
    },
};
use serde::Serialize;
use tracing::{debug, error, info, warn};
use tx_bakery::{
    chain_query::{
        ChainQuery, ChainQueryError, ChainTip, EraSummary, FullTransactionOutput, Network,
        ProtocolParameters,
    },
    submitter::{Submitter, SubmitterError},
};
use url::Url;

use super::{
    api::{
        to_redeemer_tag, AcquireMempoolResponse, EvaluateTransactionParams,
        EvaluateTransactionResponse, NextTransactionResponse, OgmiosHealth, OutputReference,
        QueryLedgerStateEraSummariesResponse, QueryLedgerStateProtocolParametersResponse,
        QueryLedgerStateTipResponse, QueryLedgerStateUtxoByAddressParams,
        QueryLedgerStateUtxoByOutputReferenceParams, QueryLedgerStateUtxoResponse,
        QueryNetworkStartTimeResponse, ReleaseMempoolResponse, SubmitTransactionParams,
        SubmitTransactionResponse, TransactionCbor,
    },
    error::{OgmiosError, Result},
};

#[derive(Debug, Builder, Clone)]
#[builder(build_fn(validate = "Self::validate"))]
pub struct OgmiosClientConfig {
    #[builder(default = "Url::parse(\"http://127.0.0.1:1337\").unwrap()")]
    pub url: Url,
    #[builder(default = "Network::Testnet")]
    pub network: Network,
    #[builder(default = "90")]
    pub startup_timeout: u64,
}

impl OgmiosClientConfigBuilder {
    fn validate(&self) -> std::result::Result<(), String> {
        if let Some(url) = &self.url {
            match url.scheme() {
                "http" | "https" => Ok(()),
                scheme => Err(format!(
                    "Url scheme invalid in OgmiosConfig. Expected https/http, but got {}",
                    scheme,
                )),
            }
        } else {
            Ok(())
        }
    }
}

impl OgmiosClientConfig {
    pub fn get_ws_url(&self) -> Url {
        let mut url = self.url.clone();
        // JUSTIFICATION: We ensure that the url's scheme is either http or https,
        // so this should always success.
        url.set_scheme("ws").unwrap();
        url
    }

    pub fn get_restful_health_url(&self) -> Url {
        // JUSTIFICATION: The base url and the path are always valid.
        self.url.join("health").unwrap()
    }
}

/// Ogmios client for interacting with the blockchain
pub struct OgmiosClient {
    config: OgmiosClientConfig,
    client: WsClient,
}

impl OgmiosClient {
    pub async fn connect(config: OgmiosClientConfig) -> Result<Self> {
        let giveup_time = chrono::Local::now() + time::Duration::from_secs(config.startup_timeout);

        let base = time::Duration::from_secs(1);
        let mut attempt = 0;
        loop {
            let health = Self::check_health(&config).await;
            if health
                .as_ref()
                .map_or(false, |h| h.network_synchronization == 1.0)
            {
                let client = WsClientBuilder::default()
                    .build(&config.get_ws_url())
                    .await?;
                let client = Self { config, client };

                return Ok(client);
            } else {
                if chrono::Local::now() > giveup_time {
                    return match health {
                        Err(err) => Err(OgmiosError::StartupError(anyhow!(
                            "health request failed: {:?}",
                            err
                        ))),
                        Ok(health) => Err(OgmiosError::StartupError(anyhow!(
                            "couldn't sync: {:?}",
                            health
                        ))),
                    };
                }

                // Simple exponential backoff
                let wait_duration = base
                    .checked_mul(2u32.pow(attempt))
                    .ok_or(OgmiosError::StartupError(anyhow!("cannot wait any longer")))?;
                tokio::time::sleep(wait_duration).await;
                attempt += 1;
            }
        }
    }

    pub fn get_config(&self) -> &OgmiosClientConfig {
        &self.config
    }

    async fn acquire_mempool(&self) -> Result<u64> {
        let resp: AcquireMempoolResponse = self.request("acquireMempool", rpc_params![]).await?;
        Ok(resp.slot)
    }

    async fn release_mempool(&self) -> Result<ReleaseMempoolResponse> {
        self.request("releaseMempool", rpc_params![]).await
    }

    async fn next_transaction(&self) -> Result<NextTransactionResponse> {
        let mut params = ObjectParams::new();
        params.insert("fields", ()).unwrap();
        self.request("nextTransaction", params).await
    }

    // TODO: hasTransaction returns a false even when there's a transaction in mempool
    // Bug report: https://github.com/CardanoSolutions/ogmios/issues/376
    // async fn has_transaction(&self, transaction_hash: &TransactionHash) -> Result<bool> {
    //     let params = TransactionId::from(transaction_hash);
    //     self.request("hasTransaction", params).await
    // }

    /// Make a request to ogmios JSON RPC
    /// Ogmios slightly deviates from the JSON RPC standard, so I couldn't use a 3rd party library
    /// for this
    async fn request<P, U>(&self, method: &str, params: P) -> Result<U>
    where
        U: serde::de::DeserializeOwned + Serialize,
        P: ToRpcParams + Send,
    {
        self.client.request(method, params).await.map_err(|err| {
            debug!(%err, "Ogmios JSON RPC call error.");
            OgmiosError::JSONRpcError(err)
        })
    }

    pub async fn check_health(config: &OgmiosClientConfig) -> Result<OgmiosHealth> {
        Ok(reqwest::Client::new()
            .get(config.get_restful_health_url())
            .send()
            .await?
            .json::<OgmiosHealth>()
            .await?)
    }
}

impl ChainQuery for OgmiosClient {
    fn get_network(&self) -> Network {
        self.config.network.clone()
    }

    async fn query_system_start(&self) -> std::result::Result<DateTime<Utc>, ChainQueryError> {
        let resp: QueryNetworkStartTimeResponse = self
            .request("queryNetwork/startTime", rpc_params![])
            .await?;

        Ok(DateTime::parse_from_rfc3339(&resp)
            .map_err(|source| OgmiosError::ConversionError {
                label: "SystemStart datetime".to_string(),
                source: anyhow!(source),
            })?
            .to_utc())
    }

    async fn query_era_summaries(&self) -> std::result::Result<Vec<EraSummary>, ChainQueryError> {
        let resp: QueryLedgerStateEraSummariesResponse = self
            .request("queryLedgerState/eraSummaries", rpc_params![])
            .await?;

        Ok(resp
            .into_iter()
            .map(EraSummary::try_from)
            .collect::<Result<_>>()?)
    }

    /// Query protocol parameters and cost models for all languages
    async fn query_protocol_params(
        &self,
    ) -> std::result::Result<ProtocolParameters, ChainQueryError> {
        let resp: QueryLedgerStateProtocolParametersResponse = self
            .request("queryLedgerState/protocolParameters", rpc_params![])
            .await?;

        Ok(resp.try_into()?)
    }

    /// Query current last slot of the chain
    async fn query_tip(&self) -> std::result::Result<ChainTip, ChainQueryError> {
        let resp: QueryLedgerStateTipResponse =
            self.request("queryLedgerState/tip", rpc_params![]).await?;
        Ok(resp.into())
    }

    async fn query_utxos_by_addr(
        &self,
        address: &Address,
    ) -> std::result::Result<BTreeMap<TransactionInput, FullTransactionOutput>, ChainQueryError>
    {
        debug!(?address, "Query UTxOs by address");
        let addr: csl::Address = address
            .with_extra_info(self.config.network.to_network_id())
            .try_to_csl()
            .map_err(OgmiosError::TryFromPLAError)?;

        let addr = addr.to_bech32(Some("addr".to_owned())).map_err(|source| {
            OgmiosError::ConversionError {
                label: "Address to Bech32".to_string(),
                source: anyhow!(source),
            }
        })?;
        let params = QueryLedgerStateUtxoByAddressParams {
            addresses: vec![addr.to_string()],
        };

        let resp: QueryLedgerStateUtxoResponse =
            self.request("queryLedgerState/utxo", params).await?;

        decode_query_ledger_state_utxo_response(resp)
    }

    async fn query_utxos_by_ref(
        &self,
        references: Vec<&TransactionInput>,
    ) -> std::result::Result<BTreeMap<TransactionInput, FullTransactionOutput>, ChainQueryError>
    {
        let output_references = references
            .into_iter()
            .cloned()
            .map(OutputReference::try_from)
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let params = QueryLedgerStateUtxoByOutputReferenceParams { output_references };

        let resp: QueryLedgerStateUtxoResponse =
            self.request("queryLedgerState/utxo", params).await?;

        decode_query_ledger_state_utxo_response(resp)
    }
}

impl Submitter for OgmiosClient {
    /// Evaluate a transaction and return execution budgets for each script
    async fn evaluate_transaction(
        &self,
        tx_builder: &csl::TransactionBuilder,
        plutus_scripts: &[csl::PlutusScript],
        redeemers: &[csl::Redeemer],
    ) -> std::result::Result<BTreeMap<(csl::RedeemerTag, csl::BigNum), csl::ExUnits>, SubmitterError>
    {
        let mut tx_builder = tx_builder.clone();

        tx_builder.set_fee(&csl::BigNum::from(0u64));

        let mut witness_set = csl::TransactionWitnessSet::new();

        let mut script_witnesses = csl::PlutusScripts::new();

        plutus_scripts
            .iter()
            .for_each(|script| script_witnesses.add(script));

        let mut redeemer_witnesses = csl::Redeemers::new();

        redeemers
            .iter()
            .for_each(|redeemer| redeemer_witnesses.add(redeemer));

        witness_set.set_plutus_scripts(&script_witnesses);
        witness_set.set_redeemers(&redeemer_witnesses);

        let tx_body = tx_builder.build().map_err(|err| {
            error!(%err, "Transaction builder error.");
            SubmitterError(anyhow::anyhow!("Transaction builder error: {}", err))
        })?;
        let tx = csl::Transaction::new(&tx_body, &witness_set, None);

        debug!("Evaluating transaction");
        let params = EvaluateTransactionParams {
            transaction: TransactionCbor { cbor: tx.to_hex() },
            additional_utxo: Vec::new(),
        };

        let resp: EvaluateTransactionResponse = self.request("evaluateTransaction", params).await?;

        resp.into_iter()
            .map(|budgets| {
                Ok((
                    (
                        to_redeemer_tag(&budgets.validator.purpose)?,
                        csl::BigNum::from(budgets.validator.index),
                    ),
                    csl::ExUnits::new(
                        &csl::BigNum::from(budgets.budget.memory),
                        &csl::BigNum::from(budgets.budget.cpu),
                    ),
                ))
            })
            .collect()
    }

    async fn submit_transaction(
        &self,
        tx: &csl::FixedTransaction,
    ) -> std::result::Result<TransactionHash, SubmitterError> {
        debug!("Submitting transaction");
        let params = SubmitTransactionParams {
            transaction: TransactionCbor { cbor: tx.to_hex() },
            additional_utxo: Vec::new(),
        };

        let resp: Result<SubmitTransactionResponse> =
            self.request("submitTransaction", params).await;

        Ok(resp?.transaction.try_into()?)
    }

    async fn await_tx_confirm(
        &self,
        tx_hash: &TransactionHash,
    ) -> std::result::Result<(), SubmitterError> {
        info!(?tx_hash, "Awaiting transaction confirmation.");
        let do_wait = || async {
            loop {
                let _ = self.acquire_mempool().await?;

                // TODO: hasTransaction returns a false even when there's a transaction in mempool
                // Bug report: https://github.com/CardanoSolutions/ogmios/issues/376
                //
                // let has_tx = self.has_transaction(tx_hash).await?;
                let mut has_tx = false;
                while let NextTransactionResponse::TransactionId {
                    transaction: Some(resp),
                } = self.next_transaction().await?
                {
                    has_tx = has_tx || resp == tx_hash.into();

                    if has_tx {
                        break;
                    }
                }

                if !has_tx {
                    let _ = self.release_mempool().await?;
                    return Result::Ok(());
                }
            }
        };

        let mut retry_counter = 0;

        while retry_counter < 5 {
            match do_wait().await {
                Ok(_) => return Ok(()),
                Err(err) => warn!(
                    "Unable to confirm transaction {:?}: {}, retrying",
                    tx_hash, err
                ),
            }

            retry_counter += 1;
        }

        Err(SubmitterError(anyhow!(
            "Unable to confirm transaction {:?}",
            tx_hash
        )))
    }
}

fn decode_query_ledger_state_utxo_response(
    resp: QueryLedgerStateUtxoResponse,
) -> std::result::Result<BTreeMap<TransactionInput, FullTransactionOutput>, ChainQueryError> {
    resp.iter()
        .map(|utxo| {
            Ok((
                TransactionInput::try_from(utxo).map_err(|source| {
                    OgmiosError::ConversionError {
                        label: "TransactionInput".to_string(),
                        source: anyhow!(source),
                    }
                })?,
                FullTransactionOutput::try_from(utxo).map_err(|source| {
                    OgmiosError::ConversionError {
                        label: "TransactionInput".to_string(),
                        source: anyhow!(source),
                    }
                })?,
            ))
        })
        .collect()
}
