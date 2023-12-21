use std::str::FromStr;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};

#[subxt::subxt(runtime_metadata_path = "./metadata/azero-mainnet.scale")]
pub mod azero_11_4 {}

#[subxt::subxt(runtime_metadata_path = "./metadata/azero-12.0.scale")]
pub mod azero_12_0 {}

pub type Client = OnlineClient<PolkadotConfig>;
pub type BlockHash = <PolkadotConfig as subxt::Config>::Hash;

pub mod contracts;
mod psp22;
pub mod storage_calls;
pub mod token_db;

pub const MAINNET_TOKEN_DB_FILEPATH_JSON: &str = "mainnet_token_db.json";
pub const TESTNET_TOKEN_DB_FILEPATH_JSON: &str = "testnet_token_db.json";

pub const WS_AZERO_MAINNET: &str = "wss://ws.azero.dev:443";
pub const WS_AZERO_TESTNET: &str = "wss://ws.test.azero.dev:443";

pub const ALICE: &str = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

pub fn alice_acc() -> AccountId32 {
    AccountId32::from_str(ALICE).unwrap()
}

pub async fn initialize_client(url: &str) -> Client {
    loop {
        match Client::from_url(url).await {
            Ok(client) => break client,
            Err(e) => {
                println!("Error {} initializing client at {}", e, url);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}
