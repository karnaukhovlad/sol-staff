use utils::read_config;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

const CONFIG_PATH: &str = "./data/config.yaml";

fn main() {
    let config = read_config(CONFIG_PATH).expect("Failed to read config");
    let client = RpcClient::new(config.rpc_url.clone());
    for wallet in config.wallets {
        match wallet.parse::<Pubkey>() {
            Ok(pubkey) => match client.get_balance(&pubkey) {
                Ok(lamports) => println!("{}: {} lamports", pubkey, lamports),
                Err(e) => println!("{}: error: {}", pubkey, e),
            },
            Err(e) => println!("{}: invalid pubkey: {}", wallet, e),
        }
    }
}
