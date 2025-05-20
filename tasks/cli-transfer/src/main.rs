use utils::read_config;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::{Signer, read_keypair_file}, pubkey::Pubkey, system_instruction, transaction::Transaction};
use serde::Deserialize;
use std::time::Instant;
use futures::future::join_all;
use std::sync::Arc;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use clap::{Parser, Subcommand, Args};

#[derive(Debug, Deserialize)]
pub struct TransferConfig {
    pub source_wallets: Vec<String>, // paths to keypair files
    pub destination_wallets: Vec<String>, // pubkey strings
    pub rpc_url: String,
    pub amount: u64, // in lamports
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run SOL transfers from config file
    Transfer(TransferArgs),
    // Future: Add more subcommands here
}

#[derive(Args, Debug)]
struct TransferArgs {
    /// Path to the config YAML file
    #[arg(short, long, default_value = "./data/cli-transfer-config.yaml")]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Transfer(args) => {
            if let Err(e) = run_transfers(&args.config).await {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

async fn run_transfers(config_path: &str) -> anyhow::Result<()> {
    let config: TransferConfig = read_config(config_path)?;
    let client = Arc::new(RpcClient::new_with_commitment(config.rpc_url.clone(), CommitmentConfig{ commitment: CommitmentLevel::Confirmed }));
    let amount = config.amount;
    let start = Instant::now();

    let mut tasks: Vec<tokio::task::JoinHandle<anyhow::Result<(String, String, Option<solana_sdk::signature::Signature>, std::time::Duration)>>> = vec![];
    for (src_path, dst_str) in config.source_wallets.iter().zip(config.destination_wallets.iter()) {
        let client = Arc::clone(&client);
        let src_path = src_path.clone();
        let dst_str = dst_str.clone();
        let amount = amount;
        let task = tokio::spawn(async move {
            let from = read_keypair_file(&src_path).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let to = dst_str.parse::<Pubkey>()?;
            let attempt_start = Instant::now();
            let ix = system_instruction::transfer(&from.pubkey(), &to, amount);
            let recent_blockhash = match client.get_latest_blockhash().await {
                Ok(b) => b,
                Err(e) => {
                    println!("{} -> {}: failed to get blockhash: {} (elapsed: {:?})", src_path, dst_str, e, attempt_start.elapsed());
                    return Ok((src_path, dst_str, None, attempt_start.elapsed()));
                }
            };
            let tx = Transaction::new_signed_with_payer(
                &[ix],
                Some(&from.pubkey()),
                &[&from],
                recent_blockhash,
            );

            let send_result = client.send_and_confirm_transaction(&tx).await;
            match send_result {
                Ok(sig) => {
                    let elapsed = attempt_start.elapsed();
                    return Ok((src_path, dst_str, Some(sig), elapsed));
                },
                Err(e) => {
                    println!("{} -> {}: failed to send tx: {} (elapsed: {:?})", src_path, dst_str, e, attempt_start.elapsed());
                    return Ok((src_path, dst_str, None, attempt_start.elapsed()));
                }
            }
        });
        tasks.push(task);
    }

    let results = join_all(tasks).await;
    let elapsed = start.elapsed();
    println!("All {} transfers complete in {:?}", results.len(), elapsed);
    for res in results {
        match res {
            Ok(Ok((src, dst, Some(sig), duration))) => println!("RESULT: {} -> {}: Sign: {} (elapsed: {:?})", src, dst, sig, duration),
            Ok(Ok((src, dst, None, _))) => println!("RESULT: {} -> {}: FAILED", src, dst),
            Ok(Err(e)) => println!("A transfer task failed: {e}"),
            Err(_) => println!("A task panicked!"),
        }
    }
    println!("Exited");
    Ok(())
}
