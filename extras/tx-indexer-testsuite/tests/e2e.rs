#[cfg(test)]
mod e2e_tests {
    use num_bigint::BigInt;
    use oura::utils::ChainWellKnownInfo;
    use plutus_ledger_api::{
        json::Json,
        plutus_data::IsPlutusData,
        v3::{
            crypto::LedgerBytes,
            datum::OutputDatum,
            redeemer::Redeemer,
            transaction::{TransactionHash, TransactionInfo, TransactionOutput, TxInInfo},
            value::{AssetClass, CurrencySymbol, TokenName, Value},
        },
    };
    use serial_test::serial;
    use std::{collections::BTreeMap, sync::mpsc};
    use tracing::Level;
    use tx_bakery::{
        chain_query::{ChainQuery, Network},
        submitter::Submitter,
        tx_info_builder::TxScaffold,
        utils::{key_wallet::KeyWallet, script::ScriptOrRef},
        wallet::Wallet,
        ChangeStrategy, CollateralStrategy, TxBakery, TxWithCtx,
    };
    use tx_bakery_ogmios::client::{OgmiosClient, OgmiosClientConfigBuilder};
    use tx_indexer::{
        config::{NetworkConfig, NodeAddress, TxIndexerConfig},
        error::{ErrorPolicy, ErrorPolicyProvider},
        filter::Filter,
        handler::{
            callback::EventHandler,
            chain_event::{ChainEvent, TransactionEventRecord},
        },
        TxIndexer,
    };
    use url::Url;

    #[tokio::test]
    #[serial]
    async fn e2e_mint() -> std::result::Result<(), oura::Error> {
        // Set up tracing logger (logs to stdout).
        let collector = tracing_subscriber::fmt()
            .with_max_level(Level::ERROR)
            // build but do not install the subscriber.
            .finish();
        tracing::subscriber::set_global_default(collector)?;

        let ogmios_client_config = OgmiosClientConfigBuilder::default()
            .url(Url::parse("http://127.0.0.1:1337").unwrap())
            .network(Network::Testnet)
            .build()
            .unwrap();
        let ogmios = OgmiosClient::connect(ogmios_client_config).await.unwrap();

        let era_summaries = ogmios.query_era_summaries().await.unwrap();
        let byron_era_summary = &era_summaries[0];
        let shelley_era_summary = &era_summaries[1];

        let (observer_sender, observer_receiver) = mpsc::channel();
        TxIndexer::run(TxIndexerConfig::cardano_node(
            ObserveHandler { observer_sender },
            NodeAddress::UnixSocket(".devnet/node.socket".to_string()),
            NetworkConfig::Config {
                magic: 42,
                chain_info: ChainWellKnownInfo {
                    byron_epoch_length: byron_era_summary.parameters.epoch_length as u32,
                    byron_slot_length: (byron_era_summary.parameters.slot_length / 1000) as u32,
                    byron_known_slot: byron_era_summary.start.slot,
                    byron_known_hash: "".to_string(),
                    byron_known_time: byron_era_summary.start.time.num_milliseconds() as u64,
                    shelley_epoch_length: shelley_era_summary.parameters.epoch_length as u32,
                    shelley_slot_length: (shelley_era_summary.parameters.slot_length / 1000) as u32,
                    shelley_known_slot: shelley_era_summary.start.slot,
                    shelley_known_hash: "".to_string(),
                    shelley_known_time: shelley_era_summary.start.time.num_milliseconds() as u64,
                    address_hrp: "addr_test".to_string(),
                    adahandle_policy: "".to_string(),
                },
            },
            None,
            0,
            Filter {
                curr_symbols: vec![Json::from_json_string(
                    "\"ec56549aaed71fba1ba7174672831cc20aac44c4a3b4607c38bed7f3\"",
                )
                .unwrap()],
            },
            Default::default(),
        ))
        .await
        .expect("Failed to spawn indexer");

        let wallet = KeyWallet::new_enterprise("./wallets/test.skey")
            .await
            .unwrap();

        let (tx_hash, tx_info) = test_mint(&wallet, &ogmios).await;

        let tx_rec = observer_receiver.recv().unwrap();
        assert_eq_tx(tx_hash, tx_info, tx_rec);

        Ok(())
    }

    fn assert_eq_tx(
        expected_hash: TransactionHash,
        expected_info: TransactionInfo,
        actual_tx: TransactionEventRecord,
    ) {
        assert_eq!(expected_hash, actual_tx.hash);
        // In the case of inputs, we don't want any extra actual inputs. Only the expected ones.
        assert_eq_vec_unsorted(
            &actual_tx.inputs,
            &expected_info
                .inputs
                .into_iter()
                .map(|x| x.reference)
                .collect::<Vec<_>>(),
        );
        // In the case of outputs, there may be extra change outputs.
        // Therefore, left: expected, right: actual
        assert_eq_vec_loose(
            &expected_info.outputs,
            // We need to ignore ada since it doesn't appear in expected tx info but
            // does in the actual transaction.
            &actual_tx
                .outputs
                .iter()
                .map(|TxInInfo { output, .. }| {
                    let mut output = output.clone();
                    output.value = output
                        .value
                        .filter(|cs, _, _| *cs != CurrencySymbol::Ada)
                        .normalize();

                    output
                })
                .collect(),
        );
        assert_eq!(expected_info.mint, actual_tx.mint);
    }

    // Ensures two vectors are equal regardless of order of elements. (i.e, they contain same elements)
    fn assert_eq_vec_unsorted<T: Eq + std::fmt::Debug>(left: &Vec<T>, right: &Vec<T>) {
        assert_eq!(left.len(), right.len());
        assert_eq_vec_loose(left, right);
    }

    // Loose equality. Ensures that all elements in `left` appear in `right`
    // This means that `right` may be a superset of `left`.
    fn assert_eq_vec_loose<T: Eq + std::fmt::Debug>(left: &Vec<T>, right: &Vec<T>) {
        /* TODO(chase): This could be more efficient with sets but not all required traits are implemented
        across the board. */
        for x in left {
            assert!(
                right.iter().any(|y| x == y),
                "Could not find any matches for {:?} in {:?}",
                x,
                right
            );
        }
    }

    async fn test_mint(
        wallet: &KeyWallet,
        ogmios: &OgmiosClient,
    ) -> (TransactionHash, TransactionInfo) {
        let mp: Vec<u8> = Json::from_json_string("\"WD8BAAAyIlMwA0kBBFtFUV0AFTM1c0ZuHN1oASQUgmKTCpmAGkkWW0VRXSBWYWxpZGF0aW9uIGZhaWxlZAAWVzk=\"").unwrap();
        let minting_policy = ScriptOrRef::from_bytes_v2(mp).unwrap().as_minting_policy();

        let own_utxos = ogmios
            .query_utxos_by_addr(&wallet.get_change_addr())
            .await
            .unwrap();

        let cur_sym = CurrencySymbol::NativeToken(minting_policy.0.clone());
        let token_name = TokenName(LedgerBytes(vec![1, 2]));

        let fee_input = own_utxos
            .iter()
            .find(|(_tx_in, tx_out)| tx_out.value.get_ada_amount() >= BigInt::from(5_000_000))
            .expect("Cannot find spendable input UTxO.");

        let mint_asset = AssetClass {
            currency_symbol: cur_sym.clone(),
            token_name: token_name.clone(),
        };

        let tx_info = TxScaffold::new()
            .add_pub_key_input(fee_input.0.clone(), fee_input.1.into())
            .add_mint(mint_asset, 1, Redeemer(BigInt::from(1234).to_plutus_data()))
            .add_output(TransactionOutput {
                address: wallet.get_change_addr(),
                value: Value::new().insert_token(&cur_sym, &token_name, &BigInt::from(1)),
                datum: OutputDatum::None,
                reference_script: None,
            })
            .build();

        let scripts = BTreeMap::from([minting_policy.1.clone().with_script_hash()]);

        let collateral_utxo = own_utxos
            .iter()
            .find(|(_tx_in, tx_out)| tx_out.value.get_ada_amount() >= BigInt::from(5_000_000))
            .expect("Cannot find spendable input UTxO.");

        let collateral = CollateralStrategy::Explicit {
            min_amount: 5_000_000,
            utxos: vec![TxInInfo {
                reference: collateral_utxo.0.clone(),
                output: collateral_utxo.1.into(),
            }],
        };

        let tx_bakery = TxBakery::init(ogmios).await.unwrap();

        let change_strategy = ChangeStrategy::Address(wallet.get_change_addr());
        let tx = TxWithCtx::new(&tx_info, &scripts, &collateral, &change_strategy);

        let tx_hash = tx_bakery
            .bake_and_deliver(ogmios, wallet, tx)
            .await
            .unwrap();

        ogmios.await_tx_confirm(&tx_hash).await.unwrap();

        (tx_hash, tx_info)
    }

    #[derive(thiserror::Error, Debug)]
    pub(crate) enum ObserveHandlerError {}

    impl ErrorPolicyProvider for ObserveHandlerError {
        fn get_error_policy(&self) -> ErrorPolicy<Self> {
            ErrorPolicy::Skip
        }
    }

    #[derive(Clone)]
    struct ObserveHandler {
        observer_sender: mpsc::Sender<TransactionEventRecord>,
    }

    impl EventHandler for ObserveHandler {
        type Error = ObserveHandlerError;

        async fn handle(&self, ev: ChainEvent) -> Result<(), Self::Error> {
            if let ChainEvent::TransactionEvent { transaction, .. } = ev {
                self.observer_sender.send(transaction).unwrap();
                Ok(())
            } else {
                Ok(())
            }
        }
    }
}
