use azero_contracts::storage::{get_contract_storage_from_address, get_contract_storage_root_from_address, ContractStorage};
use anyhow::Result;
use azero_config::{AccountId, BlockHash, RpcClient};
use codec::Decode;
use primitive_types::U256;
use std::str::FromStr;


use std::collections::{BTreeMap, BTreeSet};


type CodecPairsKey = (AccountId, AccountId);
type CodecPairsValue = (AccountId, u8);
const PAIRS_PREFIX_HEX: &str = "e3d42e90";

#[derive(Decode)]
struct CodecRouterContract {
	factory: AccountId,
	wnative: AccountId,
	owner: AccountId,
}

#[derive(Debug, Clone)]
pub struct Router {
	pub factory: AccountId,
	pub wnative: AccountId,
	pub owner: AccountId,
	pub pairs: BTreeMap<(AccountId, AccountId), (AccountId, u8)>,
}

impl Router {
	pub fn from_storage(storage: ContractStorage) -> Self {
		let mut pairs = BTreeMap::new();
		let pair_prefix = hex::decode(PAIRS_PREFIX_HEX).unwrap();
		let root_v = storage.get([0, 0, 0, 0].as_ref()).unwrap();
		let root_storage = CodecRouterContract::decode(&mut &root_v[..]).unwrap();
		for (k, v) in storage.iter() {
			if k.starts_with(pair_prefix.as_slice()) {
				let (pair, fee) = CodecPairsValue::decode(&mut &v[..]).unwrap();
				let (t0, t1) = CodecPairsKey::decode(&mut &k[pair_prefix.len()..]).unwrap();
				pairs.insert((t0, t1), (pair, fee));
				continue;
			}
		}
		Router {
			factory: root_storage.factory,
			wnative: root_storage.wnative,
			owner: root_storage.owner,
			pairs,
		}
	}

	pub fn get_pool_addresses(&self) -> Vec<AccountId> {
		self.pairs
			.values()
			.map(|(pool, _)| pool.clone())
			.collect::<BTreeSet<_>>()
			.into_iter()
			.collect()
	}
}



#[derive(Decode)]
struct CodecPSP22Data {
	#[allow(dead_code)]
	total_supply: u128,
}

#[derive(Decode)]
struct CodecPairContract {
	#[allow(dead_code)]
	psp22: CodecPSP22Data,
	pair: CodecPairData,
}
#[derive(Decode)]
struct CodecPairData {
	#[allow(dead_code)]
	factory: AccountId,
	token_0: AccountId,
	token_1: AccountId,
	reserve_0: u128,
	#[allow(dead_code)]
	reserve_1: u128,
	#[allow(dead_code)]
	block_timestamp_last: u32,
	#[allow(dead_code)]
	price_0_cumulative_last: U256,
	#[allow(dead_code)]
	price_1_cumulative_last: U256,
	#[allow(dead_code)]
	k_last: Option<U256>,
	fee: u8,
}

#[derive(Debug, Clone)]
pub struct Pair {
	pub address: AccountId,
	pub tokens: Vec<AccountId>,
	pub reserves: Vec<u128>,
	pub fee: u8,
}

impl From<(AccountId, CodecPairData)> for Pair {
	fn from(pair_data: (AccountId, CodecPairData)) -> Pair {
		Pair {
			address: pair_data.0,
			tokens: vec![pair_data.1.token_0, pair_data.1.token_1],
			reserves: vec![pair_data.1.reserve_0, pair_data.1.reserve_1],
			fee: pair_data.1.fee,
		}
	}
}

impl Pair {
	pub async fn from_pool_address(
		rpc_client: &RpcClient,
		address: AccountId,
		maybe_block_hash: Option<BlockHash>,
	) -> Result<Pair> {
		let codec_pair: Option<CodecPairContract> =
			get_contract_storage_root_from_address(rpc_client, &address, maybe_block_hash).await?;
		let codec_pair = codec_pair.ok_or(anyhow::anyhow!("No pair data for {}", address))?;
		Ok((address.clone(), codec_pair.pair).into())
	}
}


const ROUTER_ADDRESS: &str = "5DRnWewtFkLtuKT6pD7QVto4fXSEjoGvX6pccjVpdCpaz2EV";

pub fn router_account_id() -> AccountId {
	AccountId::from_str(ROUTER_ADDRESS).unwrap()
}

pub async fn get_pools(rpc_client: &RpcClient, at: Option<BlockHash>) -> Result<Vec<Pair>> {
	let router = router_account_id();
	let storage = get_contract_storage_from_address(&rpc_client, &router, true, at).await?;
	let router = Router::from_storage(storage);
	let addresses = router.get_pool_addresses();
	let mut pools = Vec::new();
	let pool_futures = addresses
		.into_iter()
		.map(|address| Pair::from_pool_address(rpc_client, address.clone(), at));
	for pool in futures::future::join_all(pool_futures).await {
		pools.push(pool?);
	}
	Ok(pools)
}
