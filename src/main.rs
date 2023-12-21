
use azero_tools_rs::storage_calls::storage_to_balances;
use azero_tools_rs::{
    WS_AZERO_TESTNET, initialize_client,
};
use azero_tools_rs::{storage_calls::get_contract_storage_from_trie_id, contracts::info::backwards_compatible_get_contract_info};
use std::collections::HashMap;
use std::str::FromStr;
use subxt::utils::AccountId32;
#[tokio::main]
async fn main() {
    let fire = AccountId32::from_str("5FDkUXLExhgFT92UQvMQVG8H4Z4Ku4Mx9heUYpchxZMdY7LD").unwrap();
    let client = initialize_client(WS_AZERO_TESTNET).await;
    let info = backwards_compatible_get_contract_info(&client, &fire).await.unwrap().unwrap();
    let trie_id = info.trie_id;
    println!("Getting storage for contract {}", fire);
    let storage = get_contract_storage_from_trie_id(&client, trie_id, true, None).await.unwrap();
    println!("Storage: {:?}", storage);

    let storage_36_16: HashMap<Vec<u8>, Vec<u8>> = storage
        .iter()
        .filter(|(k, v)| k.len() == 36 && v.len() == 16)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let prefixes: Vec<Vec<u8>> = storage_36_16.keys().map(|k| k[..4].to_vec()).collect();
    let unique_prefixes = prefixes.iter().cloned().collect::<std::collections::HashSet<Vec<u8>>>();
    println!("Prefixes: {:?}", unique_prefixes);
    let balances = storage_to_balances(&storage);
    println!("Balances: {:?}", balances);
    for (k,v) in balances {
        println!("{}: {}", k, v);
    }


}


