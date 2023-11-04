use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use subxt::{rpc::types::Bytes, rpc_params, utils::AccountId32};

use crate::{BlockHash, Client};

pub async fn get_contract_state_root_from_trie_id(
    api: &Client,
    trie_id: Vec<u8>,
    maybe_block_hash: Option<BlockHash>,
) -> Result<Option<Vec<u8>>> {
    let mut key: Vec<u8> = Vec::from(":child_storage:default:".as_bytes());
    key.extend_from_slice(&trie_id);
    Ok(api
        .rpc()
        .storage(&key, maybe_block_hash)
        .await?
        .map(|s| s.0))
}

pub async fn get_contract_storage_from_trie_id(
    api: &Client,
    trie_id: Vec<u8>,
    omit_hash: bool,
    maybe_block_hash: Option<BlockHash>,
) -> Result<ContractStorage> {
    let mut res = HashMap::new();
    let child_storage_prefix = "0x".to_owned() + &hex::encode(":child_storage:default:".as_bytes());
    let child_trie_key = child_storage_prefix.to_owned() + &hex::encode(trie_id);
    let batch_size = 96;
    let block_hash = if let Some(block_hash) = maybe_block_hash {
        block_hash
    } else {
        api.rpc().block_hash(None).await?.unwrap()
    };
    let mut last_key: Option<String> = None;
    loop {
        let params = rpc_params![
            child_trie_key.clone(),
            "0x",
            batch_size,
            last_key,
            block_hash.clone()
        ];
        let keys: Vec<Bytes> = api.rpc().request("childstate_getKeysPaged", params).await?;

        let params = rpc_params![child_trie_key.clone(), keys.clone(), block_hash.clone()];
        let values: Vec<Bytes> = api
            .rpc()
            .request("childstate_getStorageEntries", params)
            .await?;
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
    Ok(res)
}

// pub async fn get_contract_storage(
//     api: &Client,
//     address: &AccountId32,
//     omit_hash: bool,
//     block_hash: Option<BlockHash>,
// ) -> Result<ContractStorage> {
//     let contract_info = get_contract_info(&api, address).await?;
//     let contract_info = match contract_info {
//         Some(c) => c,
//         None => return Err(anyhow::anyhow!("Contract not found")),
//     };
//     let trie_id = contract_info.trie_id.0;
//     get_contract_storage_from_trie_id(api, trie_id, omit_hash, block_hash).await
// }

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
