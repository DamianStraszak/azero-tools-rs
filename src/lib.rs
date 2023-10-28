use std::str::FromStr;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};

#[subxt::subxt(runtime_metadata_path = "./metadata/azero-mainnet.scale")]
pub mod azero {}

pub type Client = OnlineClient<PolkadotConfig>;
pub type BlockHash = <PolkadotConfig as subxt::Config>::Hash;
pub type ChainContractInfo = azero::runtime_types::pallet_contracts::storage::ContractInfo;

mod contract_calls;
mod psp22;
mod storage_calls;
pub mod token_db;

pub const WS_AZERO_MAINNET: &str = "wss://ws.azero.dev:443";

pub const ALICE: &str = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

pub fn alice_acc() -> AccountId32 {
    AccountId32::from_str(ALICE).unwrap()
}

pub async fn research() -> anyhow::Result<()> {
    let token_db = token_db::TokenDB::from_disk();
    let tracker = token_db::tracker::TokenDBTracker::new(token_db.clone())
        .await
        .unwrap();
    tokio::spawn(async move { tracker.run().await });
    loop {
        let db = token_db.clone_inner();
        println!("db len {}", db.contracts.len());
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}
