use crate::{tokens::Token, AccountId, QueryResult, U128AsDecString, COMMON_START_BLOCK};
use parking_lot::Mutex;
use r2d2::Pool as DBPool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, Result as SqliteResult};
use serde::Serialize;
use serde_with::serde_as;
use std::{path::Path, str::FromStr, sync::Arc};
use thiserror::Error;
pub const DATABASE_FILE: &str = "db/common_events.db";
const MAX_TOTAL_RESULT_SIZE: usize = 2256000;

#[derive(Debug, Clone)]
pub struct Trade {
	pub pool: AccountId,
	pub token_in: AccountId,
	pub token_out: AccountId,
	pub amount_in: u128,
	pub amount_out: u128,
	pub block_num: u32,
	pub event_index: u32,
	pub extrinsic_index: u32,
	pub origin: AccountId,
}

impl Trade {
	pub fn size(&self) -> usize {
		32 + 32 + 32 + 16 + 16 + 4 + 4 + 32
	}
}



#[serde_as]
#[derive(Debug, Clone, Serialize)]
pub struct Pool {
	pub pool: AccountId,
	pub token_0: AccountId,
	pub token_1: AccountId,
	#[serde_as(as = "U128AsDecString")]
	pub reserve_0: u128,
	#[serde_as(as = "U128AsDecString")]
	pub reserve_1: u128,
	pub fee: u8,
}

#[derive(Error, Debug)]
pub enum DbError {
	#[error("Database error: {0}")]
	DatabaseError(#[from] rusqlite::Error),
	#[error("Inconsistent block number")]
	InconsistentBlockNumber,
	#[error("Incorrected block to insert {2} in range [{0},{1}]")]
	IncorrectedBlockToInsert(u32, u32, u32),
	#[error("Queried blocks ({0}, {1}) are not in the range [{2}, {3}]")]
	BlocksNotInRange(u32, u32, u32, u32),
	#[error("Pool not found")]
	PoolNotFound,
}

pub type SharedPool = Arc<Mutex<DBPool<SqliteConnectionManager>>>;

pub fn get_shared_pool() -> SharedPool {
	let manager = SqliteConnectionManager::file(DATABASE_FILE);
	let pool = DBPool::builder().build(manager).unwrap();
	let shared_pool = Arc::new(Mutex::new(pool));
	shared_pool
}

pub fn get_connection() -> SqliteResult<Connection> {
	let conn = Connection::open(Path::new(DATABASE_FILE));
	if let Err(e) = &conn {
		log::error!("Error opening connection in get_connection: {:?}", e);
	}
	conn
}

pub fn get_connection_with_backoff() -> Connection {
	let mut sleep_secs = 0.001;
	loop {
		match get_connection() {
			Ok(conn) => return conn,
			Err(_) => {
				std::thread::sleep(std::time::Duration::from_secs_f64(sleep_secs));
				sleep_secs *= 2.0;
			},
		}
	}
}

pub fn init_db() -> SqliteResult<()> {
	let mut conn = Connection::open(Path::new(DATABASE_FILE))?;
	conn.pragma_update(None, "journal_mode", "WAL")?;
	let tx = conn.transaction()?;

	tx.execute(
		"CREATE TABLE IF NOT EXISTS trades (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pool TEXT NOT NULL,
			token_in TEXT NOT NULL,
			token_out TEXT NOT NULL,
			amount_in TEXT NOT NULL,
			amount_out TEXT NOT NULL,
			block_num INTEGER NOT NULL,
			event_index INTEGER NOT NULL,
			extrinsic_index INTEGER NOT NULL,
			origin TEXT NOT NULL
        )",
		[],
	)?;

	tx.execute(
		"CREATE TABLE IF NOT EXISTS pools (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pool TEXT NOT NULL,
			token_0 TEXT NOT NULL,
			token_1 TEXT NOT NULL,
			reserve_0 TEXT NOT NULL,
			reserve_1 TEXT NOT NULL,
			fee INTEGER NOT NULL
        )",
		[],
	)?;

	tx.execute(
		"CREATE TABLE IF NOT EXISTS tokens (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            address TEXT NOT NULL,
			name TEXT,
			symbol TEXT,
			decimals INTEGER NOT NULL
        )",
		[],
	)?;

	tx.execute(
		"CREATE TABLE IF NOT EXISTS metadata (
            id INTEGER PRIMARY KEY,
            indexed_till INTEGER NOT NULL
        )",
		[],
	)?;

	tx.execute(
		"INSERT INTO metadata (id, indexed_till)
         SELECT 1, ?
         WHERE NOT EXISTS (SELECT 1 FROM metadata WHERE id = 1)",
		params![COMMON_START_BLOCK],
	)?;

	tx.commit()?;

	Ok(())
}

pub fn get_indexed_till(conn: &Connection) -> SqliteResult<u32> {
	let mut stmt = conn.prepare("SELECT indexed_till FROM metadata WHERE id = 1")?;
	let mut rows = stmt.query([])?;
	if let Some(row) = rows.next()? {
		let indexed_till: u32 = row.get(0)?;
		Ok(indexed_till)
	} else {
		Err(rusqlite::Error::QueryReturnedNoRows.into())
	}
}

fn pool_from_row(row: &rusqlite::Row) -> rusqlite::Result<Pool> {
	let pool = {
		let pool: String = row.get(0)?;
		AccountId::from_str(&pool).unwrap()
	};
	let token_0 = {
		let token_0: String = row.get(1)?;
		AccountId::from_str(&token_0).unwrap()
	};
	let token_1 = {
		let token_1: String = row.get(2)?;
		AccountId::from_str(&token_1).unwrap()
	};
	let reserve_0: u128 = {
		let reserve_0: String = row.get(3)?;
		reserve_0.parse().unwrap()
	};
	let reserve_1: u128 = {
		let reserve_1: String = row.get(4)?;
		reserve_1.parse().unwrap()
	};
	let fee: u8 = row.get(5)?;

	Ok(Pool { pool, token_0, token_1, reserve_0, reserve_1, fee })
}

pub fn get_pool(conn: &Connection, pool: &AccountId) -> SqliteResult<Option<Pool>> {
	let mut stmt = conn.prepare(
		"SELECT pool, token_0, token_1, reserve_0, reserve_1, fee FROM pools WHERE pool = ?1",
	)?;
	let mut rows = stmt.query(params![pool.to_string()])?;
	if let Some(row) = rows.next()? {
		let pool = pool_from_row(&row)?;
		Ok(Some(pool))
	} else {
		Ok(None)
	}
}

pub fn get_pools(conn: &Connection) -> SqliteResult<Vec<Pool>> {
	let mut stmt =
		conn.prepare("SELECT pool, token_0, token_1, reserve_0, reserve_1, fee FROM pools")?;
	let mut rows = stmt.query([])?;
	let mut pools = Vec::new();
	while let Some(row) = rows.next()? {
		let pool = pool_from_row(&row)?;
		pools.push(pool);
	}
	Ok(pools)
}

pub fn get_tokens(conn: &Connection) -> SqliteResult<Vec<Token>> {
	let mut stmt = conn.prepare("SELECT address, name, symbol, decimals FROM tokens")?;
	let mut rows = stmt.query([])?;
	let mut tokens = Vec::new();
	while let Some(row) = rows.next()? {
		let address = {
			let address: String = row.get(0)?;
			AccountId::from_str(&address).unwrap()
		};
		let name: Option<String> = row.get(1)?;
		let symbol: Option<String> = row.get(2)?;
		let decimals: u8 = row.get(3)?;

		tokens.push(Token { address, name, symbol, decimals });
	}
	Ok(tokens)
}

fn insert_token_tx(tx: &rusqlite::Transaction, token: &Token) -> rusqlite::Result<()> {
	tx.execute(
		"INSERT INTO tokens (
			address,
			name,
			symbol,
			decimals
		) VALUES (?1, ?2, ?3, ?4)",
		params![
			token.address.to_string(),
			token.name.clone(),
			token.symbol.clone(),
			token.decimals.clone()
		],
	)?;
	Ok(())
}

pub fn insert_token(conn: &mut Connection, token: &Token) -> SqliteResult<()> {
	let tx = conn.transaction()?;
	insert_token_tx(&tx, token)?;
	tx.commit()?;
	Ok(())
}

pub fn insert_pool(conn: &mut Connection, pool: &Pool) -> SqliteResult<()> {
	let tx = conn.transaction()?;
	insert_pool_tx(&tx, pool)?;
	tx.commit()?;
	Ok(())
}

fn insert_pool_tx(tx: &rusqlite::Transaction, pool: &Pool) -> rusqlite::Result<()> {
	tx.execute(
		"INSERT INTO pools (
			pool,
			token_0,
			token_1,
			reserve_0,
			reserve_1,
			fee
		) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
		params![
			pool.pool.to_string(),
			pool.token_0.to_string(),
			pool.token_1.to_string(),
			pool.reserve_0.to_string(),
			pool.reserve_1.to_string(),
			pool.fee
		],
	)?;
	Ok(())
}

pub fn insert_trades(
	conn: &mut Connection,
	trades: Vec<Trade>,
	block_start: u32,
	block_stop: u32,
) -> Result<(), DbError> {
	let indexed_till = get_indexed_till(conn)?;
	if block_start != indexed_till + 1 {
		return Err(DbError::InconsistentBlockNumber);
	}

	let tx = conn.transaction()?;
	for trade in trades {
		tx.execute(
			"INSERT INTO trades (
				pool,
				token_in,
				token_out,
				amount_in,
				amount_out,
				block_num,
				event_index,
				extrinsic_index,
				origin
			) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
			params![
				&trade.pool.to_string(),
				&trade.token_in.to_string(),
				&trade.token_out.to_string(),
				trade.amount_in.to_string(),
				trade.amount_out.to_string(),
				trade.block_num,
				&trade.event_index,
				trade.extrinsic_index,
				&trade.origin.to_string(),
			],
		)?;
	}
	// update indexed_till
	tx.execute("UPDATE metadata SET indexed_till = ?1 WHERE id = 1", params![block_stop])?;
	tx.commit()?;
	Ok(())
}

fn trade_from_row(row: &rusqlite::Row) -> rusqlite::Result<Trade> {
	let pool = {
		let pool: String = row.get(0)?;
		AccountId::from_str(&pool).unwrap()
	};
	let token_in = {
		let token_in: String = row.get(1)?;
		AccountId::from_str(&token_in).unwrap()
	};
	let token_out = {
		let token_out: String = row.get(2)?;
		AccountId::from_str(&token_out).unwrap()
	};
	let amount_in: u128 = {
		let amount_in: String = row.get(3)?;
		amount_in.parse().unwrap()
	};
	let amount_out: u128 = {
		let amount_out: String = row.get(4)?;
		amount_out.parse().unwrap()
	};
	let block_num: u32 = row.get(5)?;
	let event_index: u32 = row.get(6)?;
	let extrinsic_index: u32 = row.get(7)?;
	let origin = {
		let origin: String = row.get(8)?;
		AccountId::from_str(&origin).unwrap()
	};

	Ok(Trade {
		pool,
		token_in,
		token_out,
		amount_in,
		amount_out,
		block_num,
		event_index,
		extrinsic_index,
		origin,
	})
}

fn trades_from_rows(rows: &mut rusqlite::Rows) -> Result<QueryResult<Vec<Trade>>, DbError> {
	let mut trades = Vec::new();
	let mut total_size = 0;
	while let Some(row) = rows.next()? {
		if total_size > MAX_TOTAL_RESULT_SIZE {
			return Ok(QueryResult { data: trades, is_complete: false });
		}
		let trade = trade_from_row(&row)?;
		total_size += trade.size();
		trades.push(trade);
	}
	Ok(QueryResult { data: trades, is_complete: true })
}

pub fn get_trades_by_origin(
	conn: &Connection,
	block_start: u32,
	block_stop: u32,
	origin: &AccountId,
) -> Result<QueryResult<Vec<Trade>>, DbError> {
	let mut stmt = conn.prepare(
		"SELECT pool, token_in, token_out, amount_in, amount_out, block_num, event_index, extrinsic_index, origin
         FROM trades 
         WHERE block_num BETWEEN ?1 AND ?2 
         AND origin = ?3
         ORDER BY block_num ASC, event_index ASC",
	)?;
	let mut rows = stmt.query(params![block_start, block_stop, origin.to_string()])?;
	trades_from_rows(&mut rows)
}

pub fn get_trades_by_range(
	conn: &Connection,
	block_start: u32,
	block_stop: u32,
) -> Result<QueryResult<Vec<Trade>>, DbError> {
	let mut stmt = conn.prepare(
		"SELECT pool, token_in, token_out, amount_in, amount_out, block_num, event_index, extrinsic_index, origin
		 FROM trades 
		 WHERE block_num BETWEEN ?1 AND ?2 
		 ORDER BY block_num ASC, event_index ASC",
	)?;

	let mut rows = stmt.query(params![block_start, block_stop])?;
	trades_from_rows(&mut rows)
}
