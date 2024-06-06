use std::time::Instant;

use azero_config::Config;
use azero_universal::contract_events::{
	backwards_compatible_into_contract_event, GenericContractEvent,
};
use futures::channel::oneshot;
use subxt::backend::legacy::LegacyRpcMethods;

use crate::{event_db::Event, get_finalized_block_num, BlockHash, Client, RpcClient};

use super::event_db;

struct PendingRange {
	num_from: u32,
	num_to: u32,
	result: oneshot::Receiver<BlockRangeResult>,
}

struct SolvedRange {
	num_from: u32,
	num_to: u32,
	result: BlockRangeResult,
}

const MAX_SOLVED: usize = 100;
const NUM_PENDING_LEFT: usize = 25;
const NUM_PENDING_RIGHT: usize = 5;
const RANGE_SIZE: u32 = 12;

pub async fn get_hash_from_number(
	client: &RpcClient,
	num: u32,
) -> anyhow::Result<Option<BlockHash>> {
	let rpc_methods = LegacyRpcMethods::<Config>::new(client.clone());
	let n = subxt::backend::legacy::rpc_methods::NumberOrHex::Number(num as u64);
	let block_hash = rpc_methods.chain_get_block_hash(Some(n)).await?;
	Ok(block_hash)
}

struct BlockRangeResult {
	res: Vec<(u32, Vec<Event>)>,
}

async fn scrape_blocks(
	num_start: u32,
	num_end: u32,
	tx: oneshot::Sender<BlockRangeResult>,
) -> anyhow::Result<()> {
	let rpc_client = match super::get_rpc_client().await {
		Ok(client) => client,
		Err(e) => {
			log::error!("Error getting client: {}", e);
			return Err(anyhow::anyhow!("Error getting client: {}", e));
		},
	};

	let mut res = Vec::new();
	let nums = num_start..=num_end;
	let mut hashes = Vec::new();
	for num in nums.clone() {
		let block_hash = get_hash_from_number(&rpc_client, num)
			.await?
			.ok_or(anyhow::anyhow!("Block not found"))?;
		hashes.push(block_hash);
	}

	let client: Client = Client::from_rpc_client(rpc_client).await?;
	for (hash, num) in hashes.iter().zip(nums) {
		let block = client.blocks().at(*hash).await?;
		let events = match block.events().await {
			Ok(events) => events,
			Err(e) => {
				log::error!("Error getting events from block: {}", e);
				return Err(anyhow::anyhow!("Error getting events from block: {}", e));
			},
		};
		let mut contract_events = Vec::new();

		for event in events.iter() {
			match event {
				Ok(event) =>
					if let Some(e) = backwards_compatible_into_contract_event(event) {
						use GenericContractEvent::*;
						match e {
							ContractEmitted { contract, data } => {
								contract_events.push(Event {
									contract_account_id: contract,
									block_num: num,
									data,
								});
							},
							_ => {},
						}
					},
				Err(e) => {
					log::error!("Error decoding event: {}", e);
				},
			}
		}
		res.push((num, contract_events));
	}

	let _ = tx.send(BlockRangeResult { res });
	Ok(())
}

fn first_not_contained_after(bound: i32, segments: &Vec<(i32, i32)>) -> (i32, i32) {
	let mut x = bound + 1;
	for (a, b) in segments.iter() {
		if x == *a {
			x = b + 1;
		} else {
			assert!(x < *a);
			return (x, *a - 1);
		}
	}
	(x, (i32::MAX - 5))
}

fn schedule_right(
	indexed_to: u32,
	finalized_num: u32,
	pending_ranges: &Vec<(u32, u32)>,
) -> Option<(u32, u32)> {
	let mut ranges_unsigned: Vec<(i32, i32)> =
		pending_ranges.iter().map(|(a, b)| (*a as i32, *b as i32)).collect();
	ranges_unsigned.sort();
	let (a, b) = first_not_contained_after(indexed_to as i32, &ranges_unsigned);
	let b = std::cmp::min(b, a + (RANGE_SIZE as i32));
	let b = std::cmp::min(b, finalized_num as i32);
	if a > b {
		None
	} else {
		Some((a as u32, b as u32))
	}
}

fn schedule_left(
	indexed_from: u32,
	minimum_num: u32,
	pending_ranges: &Vec<(u32, u32)>,
) -> Option<(u32, u32)> {
	let mut ranges_unsigned: Vec<(i32, i32)> =
		pending_ranges.iter().map(|(a, b)| (-(*b as i32), -(*a as i32))).collect();
	ranges_unsigned.sort();
	let (a, b) = first_not_contained_after(-(indexed_from as i32), &ranges_unsigned);
	let b = std::cmp::min(b, a + (RANGE_SIZE as i32));
	let b = std::cmp::min(b, -(minimum_num as i32));
	let (a, b) = (-b, -a);
	if a > b {
		None
	} else {
		Some((a as u32, b as u32))
	}
}

pub async fn scrape() -> ! {
	let (mut indexed_from, mut indexed_to) = event_db::get_bounds().unwrap();
	println!("Indexed from: {}, to: {}", indexed_from, indexed_to);
	let mut prev_checkpoint = Instant::now();
	let mut prev_len = indexed_to + 1 - indexed_from;
	let mut pending: Vec<PendingRange> = Vec::new();
	let mut solved: Vec<SolvedRange> = Vec::new();
	let mut finalized_num = indexed_to;
	loop {
		let checkpoint = Instant::now();
		let elapsed = checkpoint.duration_since(prev_checkpoint).as_secs_f64();
		let len = indexed_to + 1 - indexed_from;
		if elapsed > 15.0 {
			let rate = (len - prev_len) as f64 / elapsed;
			println!("Indexed from: {}, to: {}, rate: {}/s", indexed_from, indexed_to, rate);
			prev_checkpoint = checkpoint;
			prev_len = len;
			finalized_num = match get_finalized_block_num().await {
				Ok(num) => num,
				Err(e) => {
					log::error!("Error getting finalized block number: {}", e);
					tokio::time::sleep(std::time::Duration::from_millis(300)).await;
					continue;
				},
			};
			println!("Finalized: {}", finalized_num);
		}

		loop {
			let mut scheduled = false;
			{
				let num_pending_right = pending.iter().filter(|p| p.num_from > indexed_to).count();
				if num_pending_right < NUM_PENDING_RIGHT && solved.len() < MAX_SOLVED {
					let all_intervals = pending
						.iter()
						.map(|p| (p.num_from, p.num_to))
						.chain(solved.iter().map(|p| (p.num_from, p.num_to)));
					let segments_right: Vec<(u32, u32)> =
						all_intervals.clone().filter(|(a, _b)| *a > indexed_to).collect();
					if let Some((a, b)) = schedule_right(indexed_to, finalized_num, &segments_right)
					{
						let (tx, rx) = oneshot::channel();
						pending.push(PendingRange { num_from: a, num_to: b, result: rx });
						tokio::spawn(scrape_blocks(a, b, tx));
						scheduled = true;
					}
				}
			}

			{
				let num_pending_left = pending.iter().filter(|p| p.num_to < indexed_from).count();

				if num_pending_left < NUM_PENDING_LEFT && solved.len() < MAX_SOLVED {
					let all_intervals = pending
						.iter()
						.map(|p| (p.num_from, p.num_to))
						.chain(solved.iter().map(|p| (p.num_from, p.num_to)));

					let segments_left: Vec<(u32, u32)> =
						all_intervals.filter(|(_a, b)| *b < indexed_from).collect();
					if num_pending_left < NUM_PENDING_LEFT && solved.len() < MAX_SOLVED {
						if let Some((a, b)) = schedule_left(indexed_from, 0, &segments_left) {
							let (tx, rx) = oneshot::channel();
							pending.push(PendingRange { num_from: a, num_to: b, result: rx });
							tokio::spawn(scrape_blocks(a, b, tx));
							scheduled = true;
						}
					}
				}
			}

			if !scheduled {
				break;
			}
		}

		pending.retain_mut(|p| match p.result.try_recv() {
			Ok(None) => true,
			Ok(Some(r)) => {
				solved.push(SolvedRange { num_from: p.num_from, num_to: p.num_to, result: r });
				false
			},
			Err(e) => {
				log::error!("Error getting result: {}", e);
				false
			},
		});
		let mut cnt = 0;
		loop {
			let ind = (0..solved.len()).find(|i| {
				solved[*i].num_from == indexed_to + 1 || solved[*i].num_to + 1 == indexed_from
			});
			if let Some(i) = ind {
				//println!("Len pending {}, len solved {}", pending.len(), solved.len());
				let s = solved.swap_remove(i);
				let to_process = if s.num_from > indexed_to {
					s.result.res
				} else {
					let mut res = s.result.res;
					res.reverse();
					res
				};
				for (num, events) in to_process {
					//println!("indexed_from {}, indexed_to {}, num {}", indexed_from, indexed_to,
					// num);
					event_db::insert_events_for_block(events, num).unwrap();
					if num > indexed_to {
						assert!(num == indexed_to + 1);
						indexed_to = num;
					} else {
						assert!(num == indexed_from - 1);
						indexed_from = num;
					}
				}
				cnt += 1;
			} else {
				break;
			}
			if cnt > 5 {
				break;
			}
		}
		if cnt == 0 {
			tokio::time::sleep(std::time::Duration::from_millis(2)).await;
		}
	}
}
