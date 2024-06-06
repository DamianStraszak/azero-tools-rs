use crate::AccountId;
use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};
use std::path::Path;
use thiserror::Error;
pub const DATABASE_FILE: &str = "db/mainnet_events.db";
const MAX_TOTAL_RESULT_SIZE: usize = 256000;

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
	pub contract_account_id: AccountId,
	pub block_num: u32,
	#[serde_as(as = "Hex")]
	pub data: Vec<u8>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct EventNoContract {
	pub block_num: u32,
	#[serde_as(as = "Hex")]
	pub data: Vec<u8>,
}

impl From<Event> for EventNoContract {
	fn from(event: Event) -> Self {
		Self { block_num: event.block_num, data: event.data }
	}
}

pub struct RawEvent {
	pub contract_account_id_raw: [u8; 32],
	pub block_num: u32,
	pub data: Vec<u8>,
}

impl From<Event> for RawEvent {
	fn from(event: Event) -> Self {
		Self {
			contract_account_id_raw: event.contract_account_id.0,
			block_num: event.block_num,
			data: event.data,
		}
	}
}

impl From<RawEvent> for Event {
	fn from(raw_event: RawEvent) -> Self {
		Self {
			contract_account_id: AccountId::from(raw_event.contract_account_id_raw),
			block_num: raw_event.block_num,
			data: raw_event.data,
		}
	}
}

#[derive(Error, Debug)]
pub enum DbError {
	#[error("Too large result")]
	TooLargeResult,
	#[error("Database error: {0}")]
	DatabaseError(#[from] rusqlite::Error),
	#[error("Inconsistent block number")]
	InconsistentBlockNumber,
	#[error("Incorrected block to insert {2} in range [{0},{1}]")]
	IncorrectedBlockToInsert(u32, u32, u32),
	#[error("Queried blocks ({0}, {1}) are not in the range [{2}, {3}]")]
	BlocksNotInRange(u32, u32, u32, u32),
}

pub fn get_bounds() -> SqliteResult<(u32, u32)> {
	let conn = {
		let c = Connection::open(Path::new(DATABASE_FILE));
		if let Err(e) = &c {
			println!("Error opening connection in get_bounds: {:?}", e);
		}
		c?
	};
	get_bounds_with_conn(&conn)
}

pub fn get_connection() -> SqliteResult<Connection> {
	let conn = Connection::open(Path::new(DATABASE_FILE));
	if let Err(e) = &conn {
		println!("Error opening connection in get_connection: {:?}", e);
	}
	conn
}

pub fn get_connection_with_backoff() -> Connection {
	let mut sleep_secs = 0.001;
	loop {
		match get_connection() {
			Ok(conn) => return conn,
			Err(e) => {
				println!("Error getting connection: {:?}", e);
				std::thread::sleep(std::time::Duration::from_secs_f64(sleep_secs));
				sleep_secs *= 2.0;
			},
		}
	}
}

pub fn get_bounds_with_conn(conn: &Connection) -> SqliteResult<(u32, u32)> {
	let mut stmt = conn.prepare("SELECT indexed_from, indexed_to FROM metadata WHERE id = 1")?;
	stmt.query_row([], |row| {
		let indexed_from: u32 = row.get(0)?;
		let indexed_to: u32 = row.get(1)?;
		Ok((indexed_from, indexed_to))
	})
}

pub fn init_db(block_num: u32) -> SqliteResult<()> {
	let mut conn = Connection::open(Path::new(DATABASE_FILE))?;
	conn.pragma_update(None, "journal_mode", "WAL")?;
	let tx = conn.transaction()?;

	tx.execute(
		"CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            contract_account_id_raw BLOB NOT NULL,
            block_num INTEGER NOT NULL,
            data BLOB NOT NULL
        )",
		[],
	)?;

	tx.execute("CREATE INDEX IF NOT EXISTS idx_block_num ON events (block_num)", [])?;
	tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_contract_account_id_raw ON events (contract_account_id_raw)",
        [],
    )?;
	tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_contract_block_num ON events (contract_account_id_raw, block_num)",
        [],
    )?;

	tx.execute(
		"CREATE TABLE IF NOT EXISTS metadata (
            id INTEGER PRIMARY KEY,
            indexed_from INTEGER NOT NULL,
            indexed_to INTEGER NOT NULL
        )",
		[],
	)?;

	tx.execute(
		"INSERT INTO metadata (id, indexed_from, indexed_to)
         SELECT 1, ?, ?
         WHERE NOT EXISTS (SELECT 1 FROM metadata WHERE id = 1)",
		params![block_num, block_num - 1],
	)?;

	tx.commit()?;

	Ok(())
}

pub fn insert_events_for_block(events: Vec<Event>, block_num: u32) -> Result<(), DbError> {
	if !events.iter().all(|e| e.block_num == block_num) {
		return Err(DbError::InconsistentBlockNumber);
	}
	let mut conn = get_connection_with_backoff();
	let (indexed_from, indexed_to) = {
		let maybe_bounds = get_bounds_with_conn(&conn);
		if let Err(e) = &maybe_bounds {
			println!("Error getting bounds in insert: {:?}", e);
		}
		maybe_bounds?
	};

	if block_num + 1 != indexed_from && indexed_to + 1 != block_num {
		return Err(DbError::IncorrectedBlockToInsert(indexed_from, indexed_to, block_num));
	}

	let tx = conn.transaction()?;

	for event in events {
		let raw_event: RawEvent = event.into();
		tx.execute(
			"INSERT INTO events (contract_account_id_raw, block_num, data) VALUES (?1, ?2, ?3)",
			params![&raw_event.contract_account_id_raw[..], raw_event.block_num, &raw_event.data],
		)?;
	}

	if block_num < indexed_from {
		tx.execute("UPDATE metadata SET indexed_from = ?1 WHERE id = 1", params![block_num])?;
	}

	if block_num > indexed_to {
		tx.execute("UPDATE metadata SET indexed_to = ?1 WHERE id = 1", params![block_num])?;
	}

	tx.commit()?;

	Ok(())
}

pub fn get_events(
	block_start: u32,
	block_stop: u32,
	contract_address: &AccountId,
	conn: &Connection,
) -> Result<Vec<Event>, DbError> {
	let (indexed_from, indexed_to) = get_bounds_with_conn(&conn)?;
	if !(block_start >= indexed_from && block_stop <= indexed_to) {
		return Err(DbError::BlocksNotInRange(indexed_from, indexed_to, block_start, block_stop));
	}
	let mut stmt = conn.prepare(
		"SELECT contract_account_id_raw, block_num, data 
        FROM events 
        WHERE block_num BETWEEN ?1 AND ?2 
        AND contract_account_id_raw = ?3
        ORDER BY block_num ASC, id ASC", // Ensure sorting by block_num and then by id
	)?;

	let contract_account_id_raw = contract_address.0;

	let mut rows = stmt.query(params![block_start, block_stop, &contract_account_id_raw[..]])?;

	let mut events = Vec::new();
	let mut total_size = 0;
	while let Some(row) = rows.next()? {
		if total_size > MAX_TOTAL_RESULT_SIZE {
			return Err(DbError::TooLargeResult);
		}

		let contract_account_id_raw: Vec<u8> = row.get(0)?;
		// We know the address is 32 bytes long so we can cast
		let array32: [u8; 32] = contract_account_id_raw.as_slice().try_into().unwrap();
		let contract_account_id = array32.into();
		let block_num: u32 = row.get(1)?;
		let data: Vec<u8> = row.get(2)?;

		total_size += data.len() + 10;

		events.push(Event { contract_account_id, block_num, data });
	}

	Ok(events)
}
