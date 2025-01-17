use anyhow::Result;
use clap::Parser;
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool},
};
use plutus_ledger_api::v3::{
    datum::OutputDatum,
    transaction::TransactionInput,
    value::{CurrencySymbol, Value},
};
use prettytable::{format, row, Table};
use std::{default::Default, fmt::Debug, path::PathBuf};
use tracing::Level;
use tx_indexer::{
    aux::{ParseAddress, ParseCurrencySymbol},
    config::{NetworkConfig, NetworkName, NodeAddress, TxIndexerConfig},
    database::diesel::sync_progress::SyncProgressTable,
    filter::Filter,
    TxIndexer,
};
use tx_indexer_testsuite::utxo_db::{handler::UtxoIndexerHandler, table::utxos::UtxosTable};

#[derive(Debug, Parser)]
struct IndexStartArgs {
    /// Cardano node socket path
    #[arg(long)]
    socket_path: String,

    /// Network name (preprod | preview | mainnet)
    #[arg(
        short('n'),
        long,
        value_parser = clap::value_parser!(NetworkName),
        required_unless_present="network",
        conflicts_with="network_magic"
    )]
    network: Option<NetworkName>,

    /// Network magic number
    #[arg(
        short('m'),
        long("magic"),
        requires = "node_config_path",
        conflicts_with = "network"
    )]

    /// Network identified by magic number and chain info file
    network_magic: Option<u64>,

    /// Cardano node configuration path
    #[arg(long)]
    node_config_path: Option<String>,

    /// Sync from this slot
    #[arg(short, long, requires = "since_block_hash")]
    since_slot: Option<u64>,

    /// Sync from this block hash
    #[arg(short('a'), long, requires = "since_slot")]
    since_block_hash: Option<String>,

    /// Filter for transactions minting this currency symbol (multiple allowed)
    #[arg(short('c'), long = "curr_symbol")]
    curr_symbols: Vec<ParseCurrencySymbol>,

    /// PostgreSQL database URL
    #[arg(
        long,
        required_unless_present = "fixture_dump_path",
        conflicts_with = "fixture_dump_path"
    )]
    postgres_url: Option<String>,

    /// Filepath of the fixture files
    #[arg(
        long,
        required_unless_present = "postgres_url",
        conflicts_with = "postgres_url"
    )]
    fixture_dump_path: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct UtxosAtArgs {
    /// Filter UTxOs by address
    #[arg(long)]
    address: ParseAddress,

    /// PostgreSQL database URL
    #[arg(long)]
    postgres_url: String,
}

#[derive(clap::Subcommand, Debug)]
enum IndexCommand {
    /// Start the indexer
    Start(IndexStartArgs),

    /// Get UTxO set
    UtxosAt(UtxosAtArgs),
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Run the Index command
    #[command(subcommand)]
    Index(IndexCommand),
}

/// Infinity Query command line interface
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,

    #[arg(long, short, global = true)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Set up tracing logger (logs to stdout).
    let collector = tracing_subscriber::fmt()
        .with_max_level(if args.debug {
            Level::DEBUG
        } else {
            Level::INFO
        })
        // build but do not install the subscriber.
        .finish();
    tracing::subscriber::set_global_default(collector)?;

    match args.command {
        Command::Index(index_cmd) => match index_cmd {
            IndexCommand::Start(IndexStartArgs {
                socket_path,
                network,
                network_magic,
                node_config_path,
                since_slot,
                since_block_hash,
                curr_symbols,
                postgres_url,
                fixture_dump_path,
            }) => {
                let network_config = network_magic
                    .map(|magic| NetworkConfig::ConfigPath {
                        magic,
                        node_config_path: node_config_path.unwrap(),
                    })
                    .or(network.map(NetworkConfig::WellKnown))
                    .unwrap();

                let (handler, sync_progress) = if let Some(postgres_url) = postgres_url {
                    let manager = ConnectionManager::<PgConnection>::new(postgres_url);

                    let pg_pool = Pool::builder()
                        .test_on_check_out(true)
                        .build(manager)
                        .expect("Could not build connection pool");

                    let mut conn = pg_pool.get().unwrap();

                    let sync_progress =
                        SyncProgressTable::get_or(&mut conn, since_slot, since_block_hash)?;

                    (UtxoIndexerHandler::postgres(pg_pool), sync_progress)
                } else {
                    (
                        UtxoIndexerHandler::fixture(fixture_dump_path.unwrap()),
                        since_slot.zip(since_block_hash),
                    )
                };

                TxIndexer::run(TxIndexerConfig::cardano_node(
                    handler,
                    NodeAddress::UnixSocket(socket_path),
                    network_config,
                    sync_progress,
                    4,
                    Filter {
                        curr_symbols: curr_symbols
                            .into_iter()
                            .map(|ParseCurrencySymbol(cur_sym)| cur_sym)
                            .collect(),
                    },
                    Default::default(),
                ))
                .await?
                .join()?;

                Ok(())
            }
            IndexCommand::UtxosAt(UtxosAtArgs {
                postgres_url,
                address,
            }) => {
                let manager = ConnectionManager::<PgConnection>::new(postgres_url);

                let pg_pool = Pool::builder()
                    .test_on_check_out(true)
                    .build(manager)
                    .expect("Could not build connection pool");

                let mut conn = pg_pool.get().unwrap();

                let utxos = UtxosTable::list_by_address(address.0, &mut conn)?;

                let mut table = Table::new();
                table.set_titles(row!["UTxO", "Datum", "Value"]);

                utxos.into_iter().for_each(|utxo| {
                    let tx_in = TransactionInput::from(utxo.utxo_ref);
                    let value_str = Value::from(utxo.value)
                        .0
                        .iter()
                        .flat_map(|(cur_sym, assets)| {
                            assets.iter().map(move |(tn, amount)| match cur_sym {
                                CurrencySymbol::Ada => amount.to_string(),
                                CurrencySymbol::NativeToken(symbol) => {
                                    format!("{:?}.{:?} {}", symbol.0, tn.0, amount)
                                }
                            })
                        })
                        .collect::<Vec<_>>()
                        .join("+");

                    table.add_row(row![
                        &format!("{:?}#{:?}", tx_in.transaction_id.0, tx_in.index),
                        &format!("{:?}", OutputDatum::try_from(utxo.datum).unwrap()),
                        &value_str,
                    ]);
                });

                table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
                table.printstd();

                Ok(())
            }
        },
    }
}
