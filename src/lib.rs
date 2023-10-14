use std::str::FromStr;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};

#[subxt::subxt(runtime_metadata_path = "./metadata/azero-mainnet.scale")]
pub mod azero {}

pub type Client = OnlineClient<PolkadotConfig>;
pub type BlockHash = <PolkadotConfig as subxt::Config>::Hash;
pub type ContractInfo = azero::runtime_types::pallet_contracts::storage::ContractInfo;

mod contract_calls;
mod psp22;
mod storage_calls;

use crate::psp22::{read_decimals, read_total_supply};

use crate::storage_calls::{get_contract_storage, storage_to_balances};

const WS_AZERO_MAINNET: &str = "wss://ws.azero.dev:443";

pub const ALICE: &str = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

pub fn alice_acc() -> AccountId32 {
    AccountId32::from_str(ALICE).unwrap()
}

pub async fn research() -> anyhow::Result<()> {
    let api = OnlineClient::<PolkadotConfig>::from_url(WS_AZERO_MAINNET).await?;
    let block_hash = api
        .rpc()
        .block_hash(Some(55555555u32.into()))
        .await
        .unwrap()
        .unwrap();
    let contract_acc =
        AccountId32::from_str("5GvYFrcAXqRM46djC2pgtp9vj1jtGJ99yv4r3RLejmBqAudL").unwrap();
    get_contract_storage(&api, contract_acc, true, None).await;
    let contract_acc =
        AccountId32::from_str("5D4doeP2gc4xeRKB4NHdGW6FNy51UbJboVMEo7QTUoqDPkJd").unwrap();
    let s = get_contract_storage(&api, contract_acc, true, Some(block_hash)).await;
    let balances = storage_to_balances(&s).into_iter().collect::<Vec<_>>();
    // balances.sort_by(|a, b| b.1.cmp(&a.1));
    // for (k, v) in balances.iter() {
    //     println!("{} {}", k, v);
    // }
    println!("balances len {}", balances.len());
    for acc in [
        "5D4doeP2gc4xeRKB4NHdGW6FNy51UbJboVMEo7QTUoqDPkJd",
        "5DGxNnuvZaCRcEQPQaJDFsqPRBvouH4cNZSpE6ERX7VJBnHn",
        "5H4aCwLKUpVpct6XGJzDGPPXFockNKQU2JUVNgUw6BXEPzST",
    ] {
        let tot = read_total_supply(&api, AccountId32::from_str(acc).unwrap()).await;
        let decimals = read_decimals(&api, AccountId32::from_str(acc).unwrap()).await;
        println!("acc {} tot {:?} dec {:?}", acc, tot, decimals);
    }
    Ok(())
}
