use crate::AccountId;
use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};
use std::path::Path;
use thiserror::Error;
pub const DATABASE_FILE: &str = "db/mainnet_events.db";
const MAX_TOTAL_RESULT_SIZE: usize = 256000;

#[derive(Debug, Serialize, Deserialize)]
pub enum EventType {
	Emitted(EmittedDetails),
	Called(CalledDetails),
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
	pub contract_account_id: AccountId,
	pub block_num: u32,
	pub event_index: u32,
	pub extrinsic_index: u32,
	pub event_type: EventType,
}

impl Event {
	fn size(&self) -> usize {
		let base = 32 + 4 + 4+ 4;
		match &self.event_type {
			EventType::Emitted(details) => base + details.data.len(),
			EventType::Called(_) => base + 32,
		}
	}

	pub fn new_emitted(contract_account_id: AccountId, block_num: u32, event_index: u32, extrinsic_index: u32, data: Vec<u8>) -> Self {
		Self {
			contract_account_id,
			block_num,
			event_index,
			extrinsic_index,
			event_type: EventType::Emitted(EmittedDetails { data }),
		}
	}

	pub fn new_called(contract_account_id: AccountId, block_num: u32, event_index: u32, extrinsic_index: u32, caller: AccountId) -> Self {
		Self {
			contract_account_id,
			block_num,
			event_index,
			extrinsic_index,
			event_type: EventType::Called(CalledDetails { caller }),
		}
	}
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct EmittedDetails {
	#[serde_as(as = "Hex")]
	pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CalledDetails {
	pub caller: AccountId,
}


pub struct DBEvent {
	pub contract_account_id: [u8; 32],
	pub block_num: u32,
	pub event_index: u32,
	pub extrinsic_index: u32,
	pub event_type: String,
	pub caller: Option<[u8; 32]>,
	pub data: Vec<u8>,
}


impl From<Event> for DBEvent {
	fn from(event: Event) -> Self {
		let contract_account_id = event.contract_account_id.0;
		let block_num = event.block_num;
		let event_index = event.event_index;
		let extrinsic_index = event.extrinsic_index;
		match event.event_type {
			EventType::Emitted(details) => Self {
				contract_account_id: contract_account_id,
				block_num,
				event_index,
				extrinsic_index,
				event_type: "emitted".to_string(),
				caller: None,
				data: details.data,
			},
			EventType::Called(details) => Self {
				contract_account_id: contract_account_id,
				block_num,
				event_index,
				extrinsic_index,
				event_type: "called".to_string(),
				caller: Some(details.caller.0),
				data: Vec::new(),
			},
		}

	}
}

impl From<DBEvent> for Event {
	fn from(event: DBEvent) -> Self {
		let contract_account_id = AccountId::from(event.contract_account_id);
		let block_num = event.block_num;
		let event_index = event.event_index;
		let extrinsic_index = event.extrinsic_index;
		let event_type = match event.event_type.as_str() {
			"emitted" => EventType::Emitted(EmittedDetails { data: event.data }),
			"called" => EventType::Called(CalledDetails { caller: AccountId::from(event.caller.unwrap()) }),
			_ => panic!("Unknown event type"),
		};
		Event {
			contract_account_id,
			block_num,
			event_index,
			extrinsic_index,
			event_type,
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
            contract_account_id BLOB NOT NULL,
            block_num INTEGER NOT NULL,
            event_index INTEGER NOT NULL,
            extrinsic_index INTEGER NOT NULL,
			event_type TEXT NOT NULL,
			caller BLOB,
            data BLOB NOT NULL,
			UNIQUE (block_num, event_index)
        )",
		[],
	)?;

	tx.execute("CREATE INDEX IF NOT EXISTS idx_block_num ON events (block_num)", [])?;
	tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_contract_account_id ON events (contract_account_id)",
        [],
    )?;
	tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_contract_block_num ON events (contract_account_id, block_num)",
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

	for event_non_raw in events {
		let event: DBEvent = event_non_raw.into();
		tx.execute(
			"INSERT INTO events (
                contract_account_id, 
                block_num, 
                event_index, 
                extrinsic_index, 
				event_type,
				caller,
                data
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
			params![
				&event.contract_account_id[..],
				event.block_num,
				event.event_index,
				event.extrinsic_index,
				event.event_type,
				event.caller,
				&event.data,
			],
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

pub fn get_events_by_contract(
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
		"SELECT contract_account_id, block_num, event_index, extrinsic_index, event_type, caller, data 
         FROM events 
         WHERE block_num BETWEEN ?1 AND ?2 
         AND contract_account_id = ?3
         ORDER BY block_num ASC, event_index ASC",
	)?;

	let contract_account_id = contract_address.0;

	let mut rows = stmt.query(params![block_start, block_stop, &contract_account_id[..]])?;

	let mut events = Vec::new();
	let mut total_size = 0;
	while let Some(row) = rows.next()? {
		if total_size > MAX_TOTAL_RESULT_SIZE {
			return Err(DbError::TooLargeResult);
		}
		let event = event_from_row(&row)?;
		total_size += event.size();
		events.push(event);
	}

	Ok(events)
}

fn event_from_row(row: &rusqlite::Row) -> rusqlite::Result<Event> {
	let contract_account_id: Vec<u8> = row.get(0)?;
	let contract_account_id: [u8; 32] = contract_account_id.as_slice().try_into().unwrap();
	let block_num: u32 = row.get(1)?;
	let event_index: u32 = row.get(2)?;
	let extrinsic_index: u32 = row.get(3)?;
	let event_type: String = row.get(4)?;
	let caller: Option<[u8; 32]> = row.get(5)?;
	let data: Vec<u8> = row.get(4)?;

	Ok(
		DBEvent {
			contract_account_id,
			block_num,
			event_index,
			extrinsic_index,
			event_type,
			caller,
			data,
		}
		.into(),
	)
}

pub fn get_events_by_range(
	block_start: u32,
	block_stop: u32,
	conn: &Connection,
) -> Result<Vec<Event>, DbError> {
	let (indexed_from, indexed_to) = get_bounds_with_conn(&conn)?;
	if !(block_start >= indexed_from && block_stop <= indexed_to) {
		return Err(DbError::BlocksNotInRange(indexed_from, indexed_to, block_start, block_stop));
	}

	let mut stmt = conn.prepare(
		"SELECT contract_account_id, block_num, event_index, extrinsic_index, event_type, caller, data 
         FROM events 
         WHERE block_num BETWEEN ?1 AND ?2 
         ORDER BY block_num ASC, event_index ASC",
	)?;

	let mut rows = stmt.query(params![block_start, block_stop])?;

	let mut events = Vec::new();
	let mut total_size = 0;
	while let Some(row) = rows.next()? {
		if total_size > MAX_TOTAL_RESULT_SIZE {
			return Err(DbError::TooLargeResult);
		}
		let event = event_from_row(&row)?;
		total_size += event.size();
		events.push(event);
	}

	Ok(events)
}
