use axum::{extract::Query, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use azero_config::AccountId;
use azero_contract_event_indexer::{
	event_db::{
		get_bounds_with_conn, get_events_by_contract, get_events_by_range, DbError, DATABASE_FILE,
	},
	start_indexer,
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

type DbPool = Pool<SqliteConnectionManager>;

#[derive(Debug, Deserialize)]
struct GetEventsParams {
	block_start: u32,
	block_stop: u32,
	contract_address: Option<AccountId>,
}

async fn handle_get_events(
	Query(params): Query<GetEventsParams>,
	db_pool: Arc<Mutex<DbPool>>,
) -> impl IntoResponse {
	let conn = {
		let pool = db_pool.lock().await;
		pool.get().unwrap()
	};
	let result = match params.contract_address {
		Some(contract_address) =>
			get_events_by_contract(params.block_start, params.block_stop, &contract_address, &conn),
		None => get_events_by_range(params.block_start, params.block_stop, &conn),
	};

	match result {
		Ok(events) => Json(events).into_response(),
		Err(DbError::BlocksNotInRange(start, stop, block_start, block_stop)) => (
			StatusCode::BAD_REQUEST,
			format!(
				"Blocks not in range, supported {}-{}, requested: {}-{}",
				start, stop, block_start, block_stop
			),
		)
			.into_response(),
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error: {}", e))
			.into_response(),
	}
}

#[derive(Debug, Serialize, Deserialize)]
struct Bounds {
	min_block: u32,
	max_block: u32,
}

async fn handle_get_status(db_pool: Arc<Mutex<DbPool>>) -> impl IntoResponse {
	let conn = {
		let pool = db_pool.lock().await;
		pool.get().unwrap()
	};
	match get_bounds_with_conn(&conn) {
		Ok(bounds) => {
			let bounds = Bounds { min_block: bounds.0, max_block: bounds.1 };
			Json(bounds).into_response()
		},
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error: {}", e))
			.into_response(),
	}
}

#[tokio::main]
async fn main() {
	tokio::spawn(async {
		start_indexer().await;
	});
	tokio::time::sleep(std::time::Duration::from_secs(1)).await; // wait for indexer to start

	let manager = SqliteConnectionManager::file(DATABASE_FILE);
	let pool = Pool::builder().build(manager).unwrap();
	let shared_pool = Arc::new(Mutex::new(pool));

	let app = Router::new()
		.route(
			"/events",
			get({
				let pool = Arc::clone(&shared_pool);
				move |query| handle_get_events(query, pool)
			}),
		)
		.route(
			"/status",
			get({
				let pool = Arc::clone(&shared_pool);
				move || handle_get_status(pool)
			}),
		);

	let addr = "0.0.0.0:3000";
	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	println!("Server running at {}", addr);
	axum::serve(listener, app.into_make_service()).await.unwrap();
}
