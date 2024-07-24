use anyhow::Result;
use azero_universal::contract_info::backwards_compatible_get_contract_info;
use codec::Decode;
use sp_core_hashing::blake2_128;
use std::collections::BTreeMap;
use subxt::{
	backend::legacy::{rpc_methods::Bytes, LegacyRpcMethods},
	rpc_params,
};

use azero_config::{AccountId, BlockHash, Client, Config, RpcClient};

pub type ContractStorage = BTreeMap<Vec<u8>, Vec<u8>>;

pub async fn get_contract_state_root_from_trie_id(
	api: &RpcClient,
	trie_id: Vec<u8>,
	maybe_block_hash: Option<BlockHash>,
) -> Result<Option<Vec<u8>>> {
	let api: subxt::backend::rpc::RpcClient = api.clone().into();
	let mut key: Vec<u8> = Vec::from(":child_storage:default:".as_bytes());
	key.extend_from_slice(&trie_id);
	let rpc = LegacyRpcMethods::<Config>::new(api.clone());
	Ok(rpc.state_get_storage(&key, maybe_block_hash).await?)
}

pub async fn get_contract_storage_from_trie_id(
	api: &RpcClient,
	trie_id: Vec<u8>,
	omit_hash: bool,
	maybe_block_hash: Option<BlockHash>,
) -> Result<ContractStorage> {
	let api: subxt::backend::rpc::RpcClient = api.clone().into();
	let mut res = BTreeMap::new();
	let child_storage_prefix = "0x".to_owned() + &hex::encode(":child_storage:default:".as_bytes());
	let child_trie_key = child_storage_prefix.to_owned() + &hex::encode(trie_id);
	let batch_size = 96;
	let block_hash = if let Some(block_hash) = maybe_block_hash {
		block_hash
	} else {
		let rpc = LegacyRpcMethods::<Config>::new(api.clone());
		rpc.chain_get_block_hash(None).await?.unwrap()
	};
	let mut last_key: Option<String> = None;
	loop {
		let params = rpc_params![child_trie_key.clone(), "0x", batch_size, last_key, block_hash];
		let keys: Vec<Bytes> = api.request("childstate_getKeysPaged", params).await?;
		let params = rpc_params![child_trie_key.clone(), keys.clone(), block_hash];
		let values: Vec<Bytes> = api.request("childstate_getStorageEntries", params).await?;
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

pub async fn get_contract_storage_key_from_trie_id(
	api: &RpcClient,
	trie_id: Vec<u8>,
	key: Vec<u8>,
	maybe_block_hash: Option<BlockHash>,
) -> Result<Option<Vec<u8>>> {
	let api: subxt::backend::rpc::RpcClient = api.clone().into();
	let child_storage_prefix = "0x".to_owned() + &hex::encode(":child_storage:default:".as_bytes());
	let child_trie_key = child_storage_prefix.to_owned() + &hex::encode(trie_id);
	let block_hash = if let Some(block_hash) = maybe_block_hash {
		block_hash
	} else {
		let rpc = LegacyRpcMethods::<Config>::new(api.clone());
		rpc.chain_get_block_hash(None).await?.unwrap()
	};
	let key: String = "0x".to_owned() + &hex::encode(blake2_128(&key)) + &hex::encode(key);
	let params = rpc_params![child_trie_key.clone(), vec![key], block_hash];
	let values: Vec<Option<Bytes>> = api.request("childstate_getStorageEntries", params).await?;
	Ok(values[0].as_ref().map(|v| v.0.clone()))
}

pub async fn get_contract_storage_from_address(
	rpc_client: &RpcClient,
	address: &AccountId,
	omit_hash: bool,
	maybe_block_hash: Option<BlockHash>,
) -> Result<ContractStorage> {
	let client = Client::from_rpc_client(rpc_client.clone()).await?;
	let info = match backwards_compatible_get_contract_info(&client, address).await? {
		Some(info) => info,
		None => return Err(anyhow::anyhow!("No contract info for {}", address)),
	};

	let trie_id = info.trie_id;
	get_contract_storage_from_trie_id(rpc_client, trie_id, omit_hash, maybe_block_hash).await
}

pub async fn get_contract_storage_key_from_address(
	rpc_client: &RpcClient,
	address: &AccountId,
	key: &Vec<u8>,
	maybe_block_hash: Option<BlockHash>,
) -> Result<Option<Vec<u8>>> {
	let client = Client::from_rpc_client(rpc_client.clone()).await?;
	let info = match backwards_compatible_get_contract_info(&client, address).await? {
		Some(info) => info,
		None => return Err(anyhow::anyhow!("No contract info for {}", address)),
	};

	let trie_id = info.trie_id;
	get_contract_storage_key_from_trie_id(rpc_client, trie_id, key.clone(), maybe_block_hash).await
}

pub async fn get_contract_raw_storage_root_from_address(
	rpc_client: &RpcClient,
	address: &AccountId,
	maybe_block_hash: Option<BlockHash>,
) -> Result<Option<Vec<u8>>> {
	let root: Vec<u8> = [0, 0, 0, 0].to_vec();
	get_contract_storage_key_from_address(rpc_client, address, &root, maybe_block_hash).await
}

pub async fn get_contract_storage_root_from_address<D: Decode>(
	rpc_client: &RpcClient,
	address: &AccountId,
	maybe_block_hash: Option<BlockHash>,
) -> Result<Option<D>> {
	let raw_bytes =
		get_contract_raw_storage_root_from_address(rpc_client, address, maybe_block_hash).await?;
	let decoded = match raw_bytes {
		Some(bytes) => D::decode(&mut &bytes[..])?,
		None => return Ok(None),
	};
	Ok(Some(decoded))
}
