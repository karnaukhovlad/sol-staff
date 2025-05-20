use std::collections::HashMap;
use yellowstone_grpc_client::{GeyserGrpcClient};
// use yellowstone_grpc_proto::geyser::{CommitmentLevel, SubscribeRequest, SubscribeRequestFilterBlocks};
use serde::Deserialize;
use std::fs;
use clap::{Parser, Subcommand, Args};
use log::error;
use tonic::codegen::tokio_stream::StreamExt;
use tonic::transport::channel::ClientTlsConfig;
// use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::prelude::*;
use yellowstone_grpc_proto::prelude::subscribe_update::UpdateOneof;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::{Signer, read_keypair_file}, pubkey::Pubkey, system_instruction, transaction::Transaction};
use tokio;

#[derive(Debug, Deserialize)]
struct Config {
    source_wallets: Vec<String>,
    destination_wallets: Vec<String>,
    rpc_url: String,
    amount: u64,
    grpc_url: String,
    grpc_token: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Subscribe to Yellowstone Geyser gRPC blocks stream
    Subscribe(SubscribeArgs),
    // Future: Add more subcommands here
}

#[derive(Args, Debug)]
struct SubscribeArgs {
    /// Path to the config YAML file
    #[arg(short, long, default_value = "data/geyser-config.yaml")]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Subscribe(args) => {
            subscribe_blocks(args).await?;
        }
    }
    Ok(())
}

async fn subscribe_blocks(args: SubscribeArgs) -> anyhow::Result<()> {
    // Load config from the specified file
    let config: Config = serde_yaml::from_str(&fs::read_to_string(&args.config)?)?;

    // Connect to Geyser gRPC (new API)
    let builder = GeyserGrpcClient::build_from_shared(
        config.grpc_url
    )?
        .x_token(Some(config.grpc_token))?
        .tls_config(ClientTlsConfig::new().with_native_roots())?;


    let mut client = builder.connect().await?;

    let commitment: CommitmentLevel = CommitmentLevel::Confirmed;


    // Subscribe to blocks
    let filter = SubscribeRequest {
        slots: HashMap::new(),
        accounts: HashMap::new(),
        transactions: HashMap::new(),
        transactions_status: HashMap::new(),
        entry: HashMap::new(),
        blocks: HashMap::new(),
        blocks_meta: HashMap::from([("".to_owned(), SubscribeRequestFilterBlocksMeta {})]),
        commitment: Some(commitment as i32),
        accounts_data_slice: vec![],
        ping: None,
        from_slot: None,
    };

    let mut stream = client.subscribe_once(filter).await?;

    println!("Subscribed to Yellowstone Geyser gRPC blocks stream...");
    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => {
                match msg.update_oneof {
                    Some(UpdateOneof::BlockMeta(block)) => {
                        let block_height = block.block_height.map(|h| h.block_height).unwrap_or_default();
                        println!("New block: {:?}", block_height);
                        tokio::spawn(async move {
                            if let Err(e) = make_transfer(block_height).await {
                                eprintln!("SOL transfer error: {e}");
                            }
                        });
                    }
                    _ => {}
                }
            }
            Err(error) => {
                error!("stream error: {error:?}");
                break;
            }
        }
    }
    Ok(())
}

async fn make_transfer(reacted_block_height: u64) -> anyhow::Result<()> {
    // Read config
    let config: Config = serde_yaml::from_str(&fs::read_to_string("data/geyser-config.yaml")?)?;
    let rpc_url = config.rpc_url;
    let amount = config.amount;
    let source_path = config.source_wallets.get(0).ok_or_else(|| anyhow::anyhow!("No source_wallets in config"))?;
    let dest_str = config.destination_wallets.get(0).ok_or_else(|| anyhow::anyhow!("No destination_wallets in config"))?;

    let client = RpcClient::new(rpc_url);
    let from = read_keypair_file(source_path).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let to = dest_str.parse::<Pubkey>()?;

    let ix = system_instruction::transfer(&from.pubkey(), &to, amount);
    let recent_blockhash = client.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&from.pubkey()),
        &[&from],
        recent_blockhash,
    );

    println!("Sending SOL transfer... | reacted to block_height: {}", reacted_block_height);
    let sig = client.send_and_confirm_transaction(&tx).await?;
    println!("SOL transfer sent! Signature: {} | reacted to block_height: {} ", sig, reacted_block_height);
    Ok(())
}
