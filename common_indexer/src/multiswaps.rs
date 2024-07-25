use std::collections::BTreeMap;

use crate::{event_db::Trade, AccountId, QueryResult, U128AsDecString};
use serde::Serialize;
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, Serialize)]
pub struct MultiSwap {
	pub origin: AccountId,
	pub token_in: AccountId,
	pub token_out: AccountId,
	pub path: Vec<AccountId>,
	#[serde_as(as = "U128AsDecString")]
	pub amount_in: u128,
	#[serde_as(as = "U128AsDecString")]
	pub amount_out: u128,
	pub block_num: u32,
	pub extrinsic_index: u32,
}

pub fn trade_result_to_multiswaps(
	result_trades: QueryResult<Vec<Trade>>,
) -> QueryResult<Vec<MultiSwap>> {
	let mut trades = result_trades.data;
	let is_complete = result_trades.is_complete;

	// We need to do the below, because if the result is incomplete, we might have a trade that is
	// not complete
	if !is_complete {
		let last_block = trades.last().unwrap().block_num;
		while trades.last().unwrap().block_num == last_block {
			trades.pop();
		}
	}
	let multiswaps = aggregate_trades(trades);
	QueryResult { data: multiswaps, is_complete }
}

fn aggregate_per_extrinsic(trades: Vec<Trade>) -> Vec<MultiSwap> {
	let origin = trades[0].origin.clone();
	if !trades.iter().all(|t| t.origin == origin) {
		log::error!("Trades with different origins in the same event");
		return Vec::new();
	}
	let mut multiswaps = Vec::new();
	let mut start_ind = 0;
	while start_ind < trades.len() {
		let mut end_ind = start_ind + 1;
		while end_ind < trades.len() &&
			trades[end_ind].token_in == trades[end_ind - 1].token_out &&
			trades[end_ind].amount_in == trades[end_ind - 1].amount_out
		{
			end_ind += 1;
		}
		let mut path = vec![trades[start_ind].token_in.clone()];
		for i in start_ind..end_ind {
			path.push(trades[i].token_out.clone());
		}
		let amount_in = trades[start_ind].amount_in;
		let amount_out = trades[end_ind - 1].amount_out;
		let multiswap = MultiSwap {
			origin: origin.clone(),
			token_in: path[0].clone(),
			token_out: path.last().unwrap().clone(),
			path,
			amount_in,
			amount_out,
			block_num: trades[start_ind].block_num,
			extrinsic_index: trades[start_ind].extrinsic_index,
		};
		multiswaps.push(multiswap);
		start_ind = end_ind;
	}
	multiswaps
}

pub fn aggregate_trades(trades: Vec<Trade>) -> Vec<MultiSwap> {
	let mut agg: BTreeMap<(u32, u32), Vec<Trade>> = BTreeMap::new();
	for trade in trades {
		let key = (trade.block_num, trade.extrinsic_index);
		agg.entry(key).or_insert_with(Vec::new).push(trade);
	}

	let mut multiswaps = Vec::new();

	for trades in agg.values() {
		let mut multiswaps_per_extrinsic = aggregate_per_extrinsic(trades.clone());
		multiswaps.append(&mut multiswaps_per_extrinsic);
	}
	multiswaps
}
