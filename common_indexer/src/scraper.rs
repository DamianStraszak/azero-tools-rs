use std::time::Instant;

use azero_config::Config;
use azero_universal::{contract_events::{
	backwards_compatible_into_contract_event, GenericContractEvent,
}, get_hash_from_number, initialize_client};
use futures::channel::oneshot;
use subxt::backend::legacy::LegacyRpcMethods;

use crate::{event_db::{get_indexed_till, Event}, get_finalized_block_num, pools::{get_pools, Pair}, BlockHash, Client, RpcClient};

use super::event_db;
use rusqlite::{params, Connection, Result as SqliteResult};



pub struct Endpoints {
	rpc: String,
	event_indexer: String,
}

pub async fn run(conn: &mut Connection, endpoints: &Endpoints) -> ! {
	let mut pool_hint = None;
	loop {
		match run_iter(conn, endpoints, &pool_hint).await {
			Ok(state) => {
				pool_hint = Some((state.pools_fetched_at, state.pools.clone()));
				todo!()
			},
			Err(e) => log::error!("Error in scraper: {:?}", e),
		}
		tokio::time::sleep(std::time::Duration::from_secs(15)).await;
	}
}

struct State {
	processed_in_iter: u32,
	to_be_processed: u32,
	pools_fetched_at: u32,
	pools: Vec<Pair>,
}

async fn get_pools_at_num(rpc_client: &RpcClient,  num: u32) -> anyhow::Result<Vec<Pair>> {
	let block_hash = match get_hash_from_number(rpc_client, num).await? {
		Some(hash) => hash,
		None => anyhow::bail!("Block {} not found", num),
	};
	let pools = get_pools(rpc_client, Some(block_hash)).await?;
	Ok(pools)
}


async fn run_iter(conn: &mut Connection, endpoints: &Endpoints, pools_hint: &Option<(u32, Vec<Pair>)>) -> anyhow::Result<State> {
	let (rpc_client, client) = initialize_client(&endpoints.rpc).await;
	let block = client.blocks().at_latest().await?;
	let block_num = block.header().number;
	let fetched_till = get_indexed_till(conn)?;
	let target_num = u32::min(fetched_till + 1000, block_num-10);
	if target_num <= fetched_till {
		log::info!("No new blocks to process");
		anyhow::bail!("No new blocks to process");
	}
	let (pools_fetched_at, pools) = {
		match pools_hint {
			Some((hint_at, hint_pools)) => {
				if *hint_at >= target_num {
					(*hint_at, hint_pools.clone())
				} else {
					let pools = get_pools_at_num(&rpc_client, block_num).await?;
					(target_num, pools)
				}
			}
			None => {
				let pools = get_pools_at_num(&rpc_client, block_num).await?;
				(block_num, pools)
			},
		}
	};


	todo!()
}