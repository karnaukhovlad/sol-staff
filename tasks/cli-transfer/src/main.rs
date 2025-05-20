use utils::read_config;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::{Signer, read_keypair_file}, pubkey::Pubkey, system_instruction, transaction::Transaction};
use serde::Deserialize;
use std::time::Instant;
use futures::future::join_all;
use std::sync::Arc;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};

#[derive(Debug, Deserialize)]
pub struct TransferConfig {
    pub source_wallets: Vec<String>, // paths to keypair files
    pub destination_wallets: Vec<String>, // pubkey strings
    pub rpc_url: String,
    pub amount: u64, // in lamports
}

const CONFIG_PATH: &str = "./data/cli-transfer-config.yaml";
#[tokio::main]
async fn main() {
    let config: TransferConfig = read_config(CONFIG_PATH).expect("Failed to read config");
    let client = Arc::new(RpcClient::new_with_commitment(config.rpc_url.clone(), CommitmentConfig{ commitment: CommitmentLevel::Confirmed }));
    let amount = config.amount;
    let start = Instant::now();

    let mut tasks = vec![];
    for (src_path, dst_str) in config.source_wallets.iter().zip(config.destination_wallets.iter()) {
        let client = Arc::clone(&client);
        let src_path = src_path.clone();
        let dst_str = dst_str.clone();
        let amount = amount;
        let task = tokio::spawn(async move {
            let from = read_keypair_file(&src_path).expect("Failed to read keypair");
            let to = dst_str.parse::<Pubkey>().expect("Invalid destination pubkey");
            let attempt_start = Instant::now();
            let ix = system_instruction::transfer(&from.pubkey(), &to, amount);
            let recent_blockhash = match client.get_latest_blockhash().await {
                Ok(b) => b,
                Err(e) => {
                    println!("{} -> {}: failed to get blockhash: {} (elapsed: {:?})", src_path, dst_str, e, attempt_start.elapsed());
                    return (src_path, dst_str, None, attempt_start.elapsed());
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
                    // println!("{} -> {}: success (elapsed: {:?}) tx: {}", src_path, dst_str, elapsed, &sig);
                    return (src_path, dst_str, Some(sig), elapsed);
                },
                Err(e) => {
                    println!("{} -> {}: failed to send tx: {} (elapsed: {:?})", src_path, dst_str, e, attempt_start.elapsed());
                    return (src_path, dst_str, None, attempt_start.elapsed());
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
            Ok((src, dst, Some(sig), duration)) => println!("RESULT: {} -> {}: Sign: {} (elapsed: {:?})", src, dst, sig, duration),
            Ok((src, dst, None, _)) => println!("RESULT: {} -> {}: FAILED", src, dst),
            Err(_) => println!("A task panicked!"),
        }
    }
    println!("Exited");
}
