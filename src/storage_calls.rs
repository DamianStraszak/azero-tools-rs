use std::collections::{BTreeMap, HashMap};

use subxt::{rpc::types::Bytes, rpc_params, utils::AccountId32};

use crate::{azero, BlockHash, Client, ContractInfo};

fn contract_info_of_key_to_account_id(key: &Vec<u8>) -> AccountId32 {
    let account_bytes = key[40..].to_vec();
    let array_u8: [u8; 32] = account_bytes.as_slice().try_into().unwrap();
    let account = AccountId32::from(array_u8);

    account
}

pub async fn get_contract_info(api: &Client, address: AccountId32) -> Option<ContractInfo> {
    let storage_address = azero::storage().contracts().contract_info_of(address);
    api.storage()
        .at_latest()
        .await
        .unwrap()
        .fetch(&storage_address)
        .await
        .unwrap()
}

pub async fn get_contract_infos(api: &Client) -> BTreeMap<AccountId32, ContractInfo> {
    let storege_address = azero::storage().contracts().contract_info_of_root();
    let mut res = BTreeMap::new();
    let mut stream = api
        .storage()
        .at_latest()
        .await
        .unwrap()
        .iter(storege_address, 200)
        .await
        .unwrap();
    while let Ok(Some((key, value))) = stream.next().await {
        let key = key.0;
        let account = contract_info_of_key_to_account_id(&key);
        res.insert(account, value);
    }
    res
}

pub async fn get_contract_storage(
    api: &Client,
    address: AccountId32,
    omit_hash: bool,
    block_hash: Option<BlockHash>,
) -> HashMap<Vec<u8>, Vec<u8>> {
    let mut res = HashMap::new();
    let contract_info = get_contract_info(&api, address).await.unwrap();
    let trie_id = contract_info.trie_id.0;
    let child_storage_prefix = "0x".to_owned() + &hex::encode(":child_storage:default:".as_bytes());
    println!(
        "trie_id {:?}, child_storage_prefix {}",
        trie_id, child_storage_prefix
    );
    let child_trie_key = child_storage_prefix.to_owned() + &hex::encode(trie_id);
    println!("child_trie_key {}", child_trie_key);
    let batch_size = 96;
    let mut last_key: Option<String> = None;
    loop {
        let params = rpc_params![
            child_trie_key.clone(),
            "0x",
            batch_size,
            last_key,
            block_hash.clone()
        ];
        let keys: Vec<Bytes> = api
            .rpc()
            .request("childstate_getKeysPaged", params)
            .await
            .unwrap();
        //println!("keys {:?} len {}", keys, keys.len());

        let params = rpc_params![child_trie_key.clone(), keys.clone(), block_hash.clone()];
        let values: Vec<Bytes> = api
            .rpc()
            .request("childstate_getStorageEntries", params)
            .await
            .unwrap();
        last_key = keys
            .last()
            .cloned()
            .map(|k| "0x".to_owned() + &hex::encode(k.0));
        let len = keys.len();
        for (k, v) in keys.into_iter().zip(values.into_iter()) {
            let key = if omit_hash { k.0[16..].to_vec() } else { k.0 };
            res.insert(key, v.0);
        }
        if len < batch_size {
            break;
        }
    }

    println!("res_len {}", res.len());

    res
}

pub type ContractStorage = HashMap<Vec<u8>, Vec<u8>>;

pub fn storage_to_balances(storage: &ContractStorage) -> BTreeMap<AccountId32, u128> {
    let magic_prefixes: Vec<Vec<u8>> = ["3b8d451d", "e4aae541", "264866c2"]
        .iter()
        .map(|s| hex::decode(s).unwrap())
        .collect();

    let storage_36_16: HashMap<Vec<u8>, Vec<u8>> = storage
        .iter()
        .filter(|(k, v)| k.len() == 36 && v.len() == 16)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let prefixes: Vec<Vec<u8>> = storage_36_16.keys().map(|k| k[..4].to_vec()).collect();
    if prefixes.is_empty() {
        return BTreeMap::new();
    }
    for magic_prefix in magic_prefixes {
        if prefixes.contains(&magic_prefix.to_owned()) {
            let mut balances = BTreeMap::new();
            for (k, v) in storage_36_16.iter() {
                if k.starts_with(&magic_prefix) {
                    //let account_hex = &k[4..];
                    let array_u8: [u8; 32] = k[4..].try_into().unwrap();
                    let account = AccountId32::from(array_u8);
                    let balance = codec::Decode::decode(&mut &v[..]).unwrap();
                    if balance > 0 {
                        balances.insert(account, balance);
                    }
                }
            }
            return balances;
        }
    }
    BTreeMap::new()
}
