#[cfg(test)]
mod plutus_database_roundtrips {
    use num_bigint::BigInt;
    use pla::goldens::v3::{
        sample_address, sample_currency_symbol, sample_datum_hash, sample_ed25519_pub_key_hash,
        sample_output_datum, sample_staking_credential, sample_token_name, sample_transaction_hash,
        sample_transaction_input, sample_transaction_output, sample_tx_in_info, sample_value,
    };
    use plutus_ledger_api as pla;
    use tx_indexer::database::plutus::*;

    mod diesel_decoding {
        use anyhow::{Context, Result};
        use diesel::{Connection, PgConnection};
        use serial_test::serial;

        use super::TestDB;

        #[test]
        #[serial]
        fn db_tests() {
            use super::*;

            let mut conn = PgConnection::establish("postgres://127.0.0.1:5555/tx_indexer").unwrap();

            clean(&mut conn).unwrap();

            let cases = [
                (currency_symbol(), "CurrencySymbol"),
                (token_name(), "TokenName"),
                (tx_hash(), "TransactionHash"),
                (pub_key_hash(), "Ed25519PubKeyHash"),
                (script_hash(), "ScriptHash"),
                (datum_hash(), "DatumHash"),
                (slot(), "Slot"),
                (plutus_data(), "PlutusData"),
                (cred(), "Credential"),
                (chain_pointer(), "ChainPointer"),
                (staking_cred(), "StakingCredential"),
                (address(), "Address"),
                (asset_quantity(), "AssetQuantity"),
                (value(), "Value"),
                (tx_in(), "TransactionInput"),
                (datum(), "OutputDatum"),
                (tx_out(), "TransactionOutput"),
                (tx_in_info(), "TxInInfo"),
            ];

            for (testdb_entry, name) in cases {
                let result = write_read(testdb_entry.clone(), &mut conn)
                    .with_context(|| name)
                    .unwrap();

                assert_eq!(
                    testdb_entry, result,
                    "{} read back from database does not equal original",
                    name
                );
            }
        }

        fn write(test: TestDB, conn: &mut PgConnection) -> Result<()> {
            use diesel::prelude::*;
            use tx_indexer_testsuite::schema::testdb;

            diesel::insert_into(testdb::table)
                .values(test)
                .execute(conn)?;

            Ok(())
        }

        fn read(conn: &mut PgConnection) -> Result<TestDB> {
            use diesel::prelude::*;

            use tx_indexer_testsuite::schema::testdb::dsl::*;

            Ok(testdb.select(TestDB::as_select()).first(conn)?)
        }

        fn clean(conn: &mut PgConnection) -> Result<()> {
            use diesel::prelude::*;

            use tx_indexer_testsuite::schema::testdb::dsl::*;

            diesel::delete(testdb).execute(conn)?;

            Ok(())
        }

        fn write_read(testdb: TestDB, conn: &mut PgConnection) -> Result<TestDB> {
            write(testdb.clone(), &mut *conn)?;
            let result = read(&mut *conn)?;
            clean(&mut *conn)?;
            Ok(result)
        }
    }

    mod sqlx_decoding {
        use anyhow::{anyhow, Context, Result};
        use serial_test::serial;
        use sqlx::{PgConnection, PgPool};

        use super::*;

        #[tokio::test]
        #[serial]
        async fn db_tests() {
            let pg_pool = PgPool::connect("postgres://127.0.0.1:5555/tx_indexer")
                .await
                .unwrap();
            let mut conn = pg_pool.acquire().await.unwrap();
            clean(&mut conn).await.unwrap();

            let cases = [
                (currency_symbol(), "CurrencySymbol"),
                (token_name(), "TokenName"),
                (tx_hash(), "TransactionHash"),
                (pub_key_hash(), "Ed25519PubKeyHash"),
                (script_hash(), "ScriptHash"),
                (datum_hash(), "DatumHash"),
                (slot(), "Slot"),
                (plutus_data(), "PlutusData"),
                (cred(), "Credential"),
                (chain_pointer(), "ChainPointer"),
                (staking_cred(), "StakingCredential"),
                (address(), "Address"),
                (asset_quantity(), "AssetQuantity"),
                (value(), "Value"),
                (tx_in(), "TransactionInput"),
                (datum(), "OutputDatum"),
                (tx_out(), "TransactionOutput"),
                (tx_in_info(), "TxInInfo"),
            ];

            for (testdb_entry, name) in cases {
                let result = write_read(testdb_entry.clone(), &mut conn)
                    .await
                    .with_context(|| name)
                    .unwrap();

                assert_eq!(
                    testdb_entry, result,
                    "{} read back from database does not equal original",
                    name
                );
            }
        }

        async fn write(test: TestDB, conn: &mut PgConnection) -> Result<()> {
            sqlx::query("INSERT INTO testdb (
              id,
              cur_sym,
              token_name,
              tx_hash,
              pub_key_hash,
              script_hash,
              datum_hash,
              slot,
              plutus_data,
              cred,
              chain_pointer,
              staking_cred,
              address,
              asset_quantity,
              value,
              tx_in,
              datum,
              tx_out,
              tx_in_info) VALUES (0, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)")
              .bind(test.cur_sym)
              .bind(test.token_name)
              .bind(test.tx_hash)
              .bind(test.pub_key_hash)
              .bind(test.script_hash)
              .bind(test.datum_hash)
              .bind(test.slot)
              .bind(test.plutus_data)
              .bind(test.cred)
              .bind(test.chain_pointer)
              .bind(test.staking_cred)
              .bind(test.address)
              .bind(test.asset_quantity)
              .bind(test.value)
              .bind(test.tx_in)
              .bind(test.datum)
              .bind(test.tx_out)
              .bind(test.tx_in_info)
            .execute(conn)
            .await?;
            Ok(())
        }

        async fn read(conn: &mut PgConnection) -> Result<TestDB> {
            sqlx::query_as("SELECT * FROM testdb")
                .fetch_one(conn)
                .await
                .map_err(|err| anyhow!(err))
        }

        async fn clean(conn: &mut PgConnection) -> Result<()> {
            sqlx::query("TRUNCATE testdb;").execute(conn).await?;
            Ok(())
        }

        async fn write_read(testdb: TestDB, conn: &mut PgConnection) -> Result<TestDB> {
            write(testdb.clone(), &mut *conn).await?;
            let result = read(&mut *conn).await?;
            clean(&mut *conn).await?;
            Ok(result)
        }
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Default,
        sqlx::FromRow,
        diesel::Queryable,
        diesel::Selectable,
        diesel::Insertable,
    )]
    #[diesel(table_name = tx_indexer_testsuite::schema::testdb)]
    pub struct TestDB {
        id: i64,
        cur_sym: Option<CurrencySymbol>,
        token_name: Option<TokenName>,
        tx_hash: Option<TransactionHash>,
        pub_key_hash: Option<Ed25519PubKeyHash>,
        script_hash: Option<ScriptHash>,
        datum_hash: Option<DatumHash>,
        slot: Option<Slot>,
        plutus_data: Option<PlutusData>,
        cred: Option<Credential>,
        chain_pointer: Option<ChainPointer>,
        staking_cred: Option<StakingCredential>,
        address: Option<Address>,
        asset_quantity: Option<AssetQuantity>,
        value: Option<Value>,
        tx_in: Option<TransactionInput>,
        datum: Option<OutputDatum>,
        tx_out: Option<TransactionOutput>,
        tx_in_info: Option<TxInInfo>,
    }

    pub(super) fn currency_symbol() -> TestDB {
        TestDB {
            cur_sym: Some(sample_currency_symbol().into()),
            ..Default::default()
        }
    }

    fn token_name() -> TestDB {
        TestDB {
            token_name: Some(sample_token_name().into()),
            ..Default::default()
        }
    }

    fn tx_hash() -> TestDB {
        TestDB {
            tx_hash: Some(sample_transaction_hash().into()),
            ..Default::default()
        }
    }

    fn pub_key_hash() -> TestDB {
        TestDB {
            pub_key_hash: Some(sample_ed25519_pub_key_hash().into()),
            ..Default::default()
        }
    }

    fn script_hash() -> TestDB {
        TestDB {
            script_hash: Some(sample_script_hash().into()),
            ..Default::default()
        }
    }

    fn datum_hash() -> TestDB {
        TestDB {
            datum_hash: Some(sample_datum_hash().into()),
            ..Default::default()
        }
    }

    fn slot() -> TestDB {
        TestDB {
            slot: Some(854321.into()),
            ..Default::default()
        }
    }

    fn plutus_data() -> TestDB {
        TestDB {
            plutus_data: Some(sample_plutus_data().try_into().unwrap()),
            ..Default::default()
        }
    }

    fn cred() -> TestDB {
        TestDB {
            cred: Some(sample_credential().into()),
            ..Default::default()
        }
    }

    fn chain_pointer() -> TestDB {
        TestDB {
            chain_pointer: Some(sample_chain_pointer().try_into().unwrap()),
            ..Default::default()
        }
    }

    fn staking_cred() -> TestDB {
        TestDB {
            staking_cred: Some(sample_staking_credential().try_into().unwrap()),
            ..Default::default()
        }
    }

    fn address() -> TestDB {
        TestDB {
            address: Some(sample_address().try_into().unwrap()),
            ..Default::default()
        }
    }

    fn asset_quantity() -> TestDB {
        TestDB {
            asset_quantity: Some(
                (
                    sample_currency_symbol(),
                    sample_token_name(),
                    BigInt::from(15470),
                )
                    .try_into()
                    .unwrap(),
            ),
            ..Default::default()
        }
    }

    fn value() -> TestDB {
        TestDB {
            value: Some(sample_value().try_into().unwrap()),
            ..Default::default()
        }
    }

    fn tx_in() -> TestDB {
        TestDB {
            tx_in: Some(sample_transaction_input().try_into().unwrap()),
            ..Default::default()
        }
    }

    fn datum() -> TestDB {
        TestDB {
            datum: Some(sample_output_datum().try_into().unwrap()),
            ..Default::default()
        }
    }

    fn tx_out() -> TestDB {
        TestDB {
            tx_out: Some(sample_transaction_output().try_into().unwrap()),
            ..Default::default()
        }
    }

    fn tx_in_info() -> TestDB {
        TestDB {
            tx_in_info: Some(sample_tx_in_info().try_into().unwrap()),
            ..Default::default()
        }
    }

    fn sample_plutus_data() -> pla::plutus_data::PlutusData {
        pla::plutus_data::PlutusData::constr(
            1,
            vec![pla::plutus_data::PlutusData::bytes(
                "Something".as_bytes().to_vec(),
            )],
        )
    }

    fn sample_script_hash() -> pla::v3::script::ScriptHash {
        pla::v3::script::ScriptHash(pla::v3::crypto::LedgerBytes([1].repeat(28).to_vec()))
    }

    fn sample_credential() -> pla::v3::address::Credential {
        pla::v3::address::Credential::Script(pla::v3::script::ValidatorHash(sample_script_hash()))
    }

    fn sample_chain_pointer() -> pla::v3::address::ChainPointer {
        pla::v3::address::ChainPointer {
            slot_number: pla::v3::address::Slot(134561.into()),
            transaction_index: pla::v3::address::TransactionIndex(4.into()),
            certificate_index: pla::v3::address::CertificateIndex(10.into()),
        }
    }
}
