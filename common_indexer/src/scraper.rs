use std::collections::{BTreeMap, BTreeSet};

use azero_contract_event_indexer::{
	event_db::{CalledDetails, EmittedDetails, Event, EventType},
	Bounds,
};
use azero_universal::{get_hash_from_number, initialize_client};
use serde::Deserialize;

use crate::{
	event_db::{get_connection_with_backoff, get_indexed_till, insert_trades, Pool, Trade},
	pools::{get_pools, Pair, PairEvent},
	tokens::{get_token_info, Token},
	AccountId, RpcClient,
};

use super::event_db;
use codec::DecodeAll;
use rusqlite::Connection;

pub struct Endpoints {
	rpc: String,
	event_indexer: String,
}

impl Endpoints {
	pub fn new(rpc: String, event_indexer: String) -> Self {
		Self { rpc, event_indexer }
	}
}

pub async fn run(endpoints: &Endpoints) -> ! {
	let conn = &mut get_connection_with_backoff();
	let mut pool_hint = None;
	loop {
		match run_iter(conn, endpoints, &pool_hint).await {
			Ok(Some(state)) => {
				log::info!(
					"Processed {} blocks, {} to go",
					state.processed_in_iter,
					state.to_be_processed
				);
				pool_hint =
					Some((state.state_fetched_at, state.pools.clone(), state.tokens.clone()));
			},
			Ok(None) => {},
			Err(e) => log::error!("Error in scraper: {:?}", e),
		}
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;
	}
}

struct State {
	processed_in_iter: u32,
	to_be_processed: u32,
	state_fetched_at: u32,
	pools: Vec<Pair>,
	tokens: Vec<Token>,
}

async fn get_pools_and_tokens_at_num(
	rpc_client: &RpcClient,
	num: u32,
) -> anyhow::Result<(Vec<Pair>, Vec<Token>)> {
	let block_hash = match get_hash_from_number(rpc_client, num).await? {
		Some(hash) => hash,
		None => anyhow::bail!("Block {} not found", num),
	};
	let pools = get_pools(rpc_client, Some(block_hash)).await?;
	let tokens = pools
		.iter()
		.map(|pool| pool.tokens.clone())
		.flatten()
		.collect::<BTreeSet<AccountId>>();
	let token_info = get_token_info(&rpc_client, tokens.iter().cloned().collect()).await?;
	let tokens = token_info.tokens();
	Ok((pools, tokens))
}

#[derive(Debug, Deserialize)]
struct GetEventsResponse {
	data: Vec<Event>,
	is_complete: bool,
}

async fn get_bounds(base_url: &str) -> anyhow::Result<Bounds> {
	let client = reqwest::Client::new();
	let url = format!("{}/status", base_url);
	let response = client.get(&url).send().await?;
	if response.status().is_success() {
		let response: Bounds = response.json().await?;
		Ok(response)
	} else {
		Err(anyhow::anyhow!("Error fetching status: {:?}", response.status()))
	}
}

async fn get_events_by_range(
	base_url: &str,
	block_start: u32,
	block_stop: u32,
) -> anyhow::Result<Vec<Event>> {
	let client = reqwest::Client::new();
	let url = format!("{}/events", base_url);
	let mut events = Vec::new();
	let mut current_start = block_start;
	loop {
		let response = client
			.get(&url)
			.query(&[("block_start", current_start), ("block_stop", block_stop)])
			.send()
			.await?;
		if response.status().is_success() {
			let response: GetEventsResponse = response.json().await?;
			events.extend(response.data);
			if response.is_complete {
				break;
			}
			current_start = events.last().unwrap().block_num;
		} else {
			let status = response.status();
			let error_text = response.text().await?;
			return Err(anyhow::anyhow!("Error fetching events: {} - {}", status, error_text));
		}
	}
	// sort by (block_num, event_index)
	events.sort_by(|a, b| {
		if a.block_num == b.block_num {
			a.extrinsic_index.cmp(&b.extrinsic_index)
		} else {
			a.block_num.cmp(&b.block_num)
		}
	});
	events.dedup();
	Ok(events)
}

fn trade_from_pair_event(
	pair_event: PairEvent,
	pool: &Pool,
	event: &Event,
	origin: AccountId,
) -> Option<Trade> {
	if let PairEvent::Swap {
		sender: _,
		amount_0_in,
		amount_1_in,
		amount_0_out,
		amount_1_out,
		to: _,
	} = &pair_event
	{
		let swap_from_index = if *amount_0_in > 0 { 0 } else { 1 };
		if swap_from_index == 0 && (*amount_0_out != 0 || *amount_1_out == 0) {
			log::error!("Invalid swap event: {:?}", pair_event);
			return None;
		}
		if swap_from_index == 1 && (*amount_1_out != 0 || *amount_0_out == 0) {
			log::error!("Invalid swap event: {:?}", pair_event);
			return None;
		}
		let amount_in = u128::max(*amount_0_in, *amount_1_in);
		let amount_out = u128::max(*amount_0_out, *amount_1_out);
		let (token_in, token_out) = if swap_from_index == 0 {
			(pool.token_0.clone(), pool.token_1.clone())
		} else {
			(pool.token_1.clone(), pool.token_0.clone())
		};
		let trade = Trade {
			pool: pool.pool.clone(),
			token_in,
			token_out,
			amount_in: amount_in.into(),
			amount_out: amount_out.into(),
			block_num: event.block_num,
			event_index: event.extrinsic_index,
			extrinsic_index: event.extrinsic_index,
			origin,
		};
		Some(trade)
	} else {
		None
	}
}

fn trades_from_events(events: Vec<Event>, pools_map: &BTreeMap<AccountId, Pool>) -> Vec<Trade> {
	let mut trades = Vec::new();
	let mut agg_events: BTreeMap<(u32, u32), Vec<Event>> = BTreeMap::new();
	for event in events {
		let key = (event.block_num, event.extrinsic_index);
		agg_events.entry(key).or_insert_with(Vec::new).push(event);
	}
	for ((_, _), events) in agg_events {
		let last = events.last().unwrap().clone();
		let origin = match last.event_type {
			EventType::Called(CalledDetails { caller, .. }) => caller,
			_ => {
				// This is instantiation -- we don't care about it
				continue;
			},
		};
		for event in events {
			if !pools_map.contains_key(&event.contract_account_id) {
				continue;
			}
			if let EventType::Emitted(EmittedDetails { data }) = &event.event_type {
				// decode to PairEvent
				let pair_event = match PairEvent::decode_all(&mut &data[..]) {
					Ok(pair_event) => pair_event,
					Err(e) => {
						log::error!("Error decoding event: {}", e);
						continue;
					},
				};
				if let Some(trade) = trade_from_pair_event(
					pair_event,
					&pools_map[&event.contract_account_id],
					&event,
					origin.clone(),
				) {
					trades.push(trade);
				}
			}
		}
	}
	trades
}

async fn fetch_trades_from_indexer(
	endpoints: &Endpoints,
	from: u32,
	to: u32,
	pools_map: &BTreeMap<AccountId, Pool>,
) -> anyhow::Result<Vec<Trade>> {
	let endpoint = endpoints.event_indexer.clone();
	let events = get_events_by_range(&endpoint, from, to).await?;
	log::info!("Fetched {} events", events.len());
	let trades = trades_from_events(events, pools_map);
	Ok(trades)
}

async fn fetch_range_from_indexer(endpoints: &Endpoints) -> anyhow::Result<Bounds> {
	let endpoint = endpoints.event_indexer.clone();
	get_bounds(&endpoint).await
}

async fn run_iter(
	conn: &mut Connection,
	endpoints: &Endpoints,
	state_hint: &Option<(u32, Vec<Pair>, Vec<Token>)>,
) -> anyhow::Result<Option<State>> {
	let (rpc_client, client) = initialize_client(&endpoints.rpc).await;
	let block = client.blocks().at_latest().await?;
	let block_num = block.header().number;
	let fetched_till = get_indexed_till(conn)?;
	let target_num = u32::min(fetched_till + 50000, block_num - 20);
	let bounds = fetch_range_from_indexer(endpoints).await?;
	let target_num = u32::min(target_num, bounds.max_block);
	if target_num <= fetched_till {
		return Ok(None);
	}
	let (state_at, pools, tokens) = {
		match state_hint {
			Some((hint_at, hint_pools, hint_tokens)) =>
				if *hint_at >= target_num {
					(*hint_at, hint_pools.clone(), hint_tokens.clone())
				} else {
					let (pools, tokens) =
						get_pools_and_tokens_at_num(&rpc_client, target_num).await?;
					(target_num, pools, tokens)
				},
			None => {
				let (pools, tokens) = get_pools_and_tokens_at_num(&rpc_client, block_num).await?;
				(block_num, pools, tokens)
			},
		}
	};
	let pools_db = event_db::get_pools(conn)?;
	let pools_map: BTreeMap<AccountId, Pool> =
		pools_db.into_iter().map(|pool| (pool.pool.clone(), pool)).collect();
	let tokens_db = event_db::get_tokens(conn)?;
	let tokens_map: BTreeMap<AccountId, Token> =
		tokens_db.into_iter().map(|token| (token.address.clone(), token)).collect();

	for pair in pools.clone() {
		if !pools_map.contains_key(&pair.address) {
			let pool = Pool {
				pool: pair.address.clone(),
				token_0: pair.tokens[0].clone(),
				token_1: pair.tokens[1].clone(),
				reserve_0: pair.reserves[0].into(),
				reserve_1: pair.reserves[1].into(),
				fee: pair.fee,
			};
			event_db::insert_pool(conn, &pool)?
		}
	}

	for token in tokens.clone() {
		if !tokens_map.contains_key(&token.address) {
			event_db::insert_token(conn, &token)?
		}
	}

	let trades =
		fetch_trades_from_indexer(endpoints, fetched_till + 1, target_num, &pools_map).await?;
	log::info!("Inserting {} trades", trades.len());
	insert_trades(conn, trades, fetched_till + 1, target_num)?;
	Ok(Some(State {
		processed_in_iter: target_num - fetched_till,
		to_be_processed: block_num - target_num,
		state_fetched_at: state_at,
		pools,
		tokens,
	}))
}
