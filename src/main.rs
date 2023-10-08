use azero::runtime_types::pallet_contracts::storage::ContractInfo;
use std::collections::BTreeMap;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};

type Client = OnlineClient<PolkadotConfig>;

#[subxt::subxt(runtime_metadata_path = "./metadata/azero-mainnet.scale")]
pub mod azero {}

fn contract_info_of_key_to_account_id(key: &Vec<u8>) -> AccountId32 {
    let account_bytes = key[40..].to_vec();
    let array_u8: [u8; 32] = account_bytes.as_slice().try_into().unwrap();
    let account = AccountId32::from(array_u8);
    account
}

async fn get_contracts(api: &Client) -> BTreeMap<AccountId32, ContractInfo> {
    let address = azero::storage().contracts().contract_info_of_root();
    let mut res = BTreeMap::new();
    let mut stream = api
        .storage()
        .at_latest()
        .await
        .unwrap()
        .iter(address, 100)
        .await
        .unwrap();
    let mut now = std::time::Instant::now();
    while let Ok(Some((key, value))) = stream.next().await {
        let key = key.0;
        let account = contract_info_of_key_to_account_id(&key);
        res.insert(account, value);
        let current = std::time::Instant::now();
        println!("{} ms", current.duration_since(now).as_millis());
        now = current;
    }
    res
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = OnlineClient::<PolkadotConfig>::from_url("wss://ws.azero.dev:443").await?;
    let cnt = get_contracts(&api).await.len();
    println!("Total {} contracts", cnt);
    Ok(())
}
