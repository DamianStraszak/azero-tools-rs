use std::collections::BTreeMap;

use anyhow::Result;
use subxt::{rpc::types::Bytes, rpc_params};

use azero_config::{BlockHash, Client};

pub type ContractStorage = BTreeMap<Vec<u8>, Vec<u8>>;

pub async fn get_contract_state_root_from_trie_id(
	api: &Client,
	trie_id: Vec<u8>,
	maybe_block_hash: Option<BlockHash>,
) -> Result<Option<Vec<u8>>> {
	let mut key: Vec<u8> = Vec::from(":child_storage:default:".as_bytes());
	key.extend_from_slice(&trie_id);
	Ok(api.rpc().storage(&key, maybe_block_hash).await?.map(|s| s.0))
}

pub async fn get_contract_storage_from_trie_id(
	api: &Client,
	trie_id: Vec<u8>,
	omit_hash: bool,
	maybe_block_hash: Option<BlockHash>,
) -> Result<ContractStorage> {
	let mut res = BTreeMap::new();
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
		let params = rpc_params![child_trie_key.clone(), "0x", batch_size, last_key, block_hash];
		let keys: Vec<Bytes> = api.rpc().request("childstate_getKeysPaged", params).await?;

		let params = rpc_params![child_trie_key.clone(), keys.clone(), block_hash];
		let values: Vec<Bytes> = api.rpc().request("childstate_getStorageEntries", params).await?;
		last_key = keys.last().cloned().map(|k| "0x".to_owned() + &hex::encode(k.0));
		let len = keys.len();
		for (k, v) in keys.into_iter().zip(values) {
			let key = if omit_hash { k.0[16..].to_vec() } else { k.0 };
			res.insert(key, v.0);
		}
		if len < batch_size {
			break;
		}
	}
	Ok(res)
}




