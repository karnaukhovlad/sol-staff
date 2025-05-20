use anchor_client::solana_sdk::{signature::read_keypair_file, signer::Signer, pubkey::Pubkey, commitment_config::CommitmentConfig};
use anchor_client::{Client, Cluster};
use clap::{Parser, Subcommand};
use anyhow::{Result, anyhow};
use std::rc::Rc;
use std::str::FromStr;
use anchor_client::solana_client::rpc_config::RpcSendTransactionConfig;
use sol_deposit;
use shellexpand;
use solana_sdk::signature::Keypair;

// Replace with your deployed program ID
const PROGRAM_ID: &str = "EL3Wpg3SVp5xqEW3SryBwmTsKBNR8Sg3VEfdvejmLMR9";

const REQUEST_CFG: RpcSendTransactionConfig = RpcSendTransactionConfig{
    skip_preflight: true,
    preflight_commitment: None,
    encoding: None,
    max_retries: None,
    min_context_slot: None,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to Solana wallet keypair
    #[arg(short, long, default_value = "~/.config/solana/id.json")]
    keypair: String,
    /// RPC URL
    #[arg(short, long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Deposit SOL into the H78pG8cVYZJa5BHqps6Qn9CiWKvzvPuYi1vrVGh37cxG
    Deposit {
        #[arg()]
        amount: f64, // in SOL
    },
    /// Withdraw SOL from the H78pG8cVYZJa5BHqps6Qn9CiWKvzvPuYi1vrVGh37cxG
    Withdraw {
        #[arg()]
        amount: f64, // in SOL
    },
    /// Check your balance in the H78pG8cVYZJa5BHqps6Qn9CiWKvzvPuYi1vrVGh37cxG
    Balance,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let payer = Rc::new(read_keypair_file(&shellexpand::tilde(&cli.keypair).to_string())
        .map_err(|e| anyhow!(e.to_string()))?);
    let cluster = Cluster::Custom(cli.rpc_url.clone(), cli.rpc_url.clone());
    let client = Client::new_with_options(cluster, payer.clone(), CommitmentConfig::confirmed());
    let program = client.program(Pubkey::from_str(PROGRAM_ID).unwrap())?;

    match cli.command {
        Commands::Deposit { amount } => {
            deposit(&program, payer, amount)?;
        }
        Commands::Withdraw { amount } => {
            withdraw(&program, payer, amount)?;
        }
        Commands::Balance => {
            check_balance(&program, payer)?;
        }
    }
    Ok(())
}

fn deposit(program: &anchor_client::Program<Rc<Keypair>>, payer: Rc<impl Signer>, amount: f64) -> Result<()> {
    use anchor_client::solana_sdk::system_program;
    use anchor_client::solana_sdk::signature::Signer as _;
    use anchor_client::solana_sdk::pubkey::Pubkey;

    let user_pubkey = payer.pubkey();
    let (user_account, _user_bump) = Pubkey::find_program_address(
        &[b"user_account", user_pubkey.as_ref()],
        &Pubkey::from_str(PROGRAM_ID).unwrap(),
    );
    let (vault, _vault_bump) = Pubkey::find_program_address(
        &[b"vault"],
        &Pubkey::from_str(PROGRAM_ID).unwrap(),
    );
    let lamports = (amount * 1_000_000_000.0) as u64;

    let tx = program
        .request()
        .accounts(anchor_lang::ToAccountMetas::to_account_metas(
            &sol_deposit::accounts::Deposit {
                user: user_pubkey,
                user_account,
                vault,
                system_program: system_program::ID,
            },
            None,
        ))
        .args(sol_deposit::instruction::Deposit { amount: lamports })
        .signer(payer.as_ref())
        .send_with_spinner_and_config(REQUEST_CFG)?;
    println!("Deposit transaction signature: {}", tx);
    Ok(())
}

fn withdraw(program: &anchor_client::Program<Rc<Keypair>>, payer: Rc<impl Signer>, amount: f64) -> Result<()> {
    use anchor_client::solana_sdk::system_program;
    use anchor_client::solana_sdk::signature::Signer as _;
    use anchor_client::solana_sdk::pubkey::Pubkey;

    let user_pubkey = payer.pubkey();
    let (user_account, _user_bump) = Pubkey::find_program_address(
        &[b"user_account", user_pubkey.as_ref()],
        &Pubkey::from_str(PROGRAM_ID).unwrap(),
    );
    let (vault, _vault_bump) = Pubkey::find_program_address(
        &[b"vault"],
        &Pubkey::from_str(PROGRAM_ID).unwrap(),
    );
    let lamports = (amount * 1_000_000_000.0) as u64;

    let tx = program
        .request()
        .accounts(anchor_lang::ToAccountMetas::to_account_metas(
            &sol_deposit::accounts::Withdraw {
                user: user_pubkey,
                user_account,
                vault,
                system_program: system_program::ID,
            },
            None,
        ))
        .args(sol_deposit::instruction::Withdraw { amount: lamports })
        .signer(payer.as_ref())
        .send_with_spinner_and_config(REQUEST_CFG)?;
    println!("Withdraw transaction signature: {}", tx);
    Ok(())
}

fn check_balance(program: &anchor_client::Program<Rc<Keypair>>, payer: Rc<impl Signer>) -> Result<()> {
    use anchor_client::solana_sdk::pubkey::Pubkey;
    let user_pubkey = payer.pubkey();
    let (user_account, _) = Pubkey::find_program_address(
        &[b"user_account", user_pubkey.as_ref()],
        &Pubkey::from_str(PROGRAM_ID).unwrap(),
    );
    let account: sol_deposit::UserAccount = program.account(user_account)?;
    println!(
        "User balance: {} lamports ({} SOL)",
        account.balance,
        account.balance as f64 / 1_000_000_000.0
    );
    Ok(())
} 