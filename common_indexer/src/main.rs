use axum::{extract::Query, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use azero_config::{AccountId, WS_AZERO_MAINNET};

use common_indexer::{
	event_db::{
		get_indexed_till, get_pools, get_shared_pool, get_tokens, get_trades_by_origin, get_trades_by_range, init_db, SharedPool
	}, multiswaps::trade_result_to_multiswaps, scraper::Endpoints, tokens::{get_price_by_token_address, Token}, COMMON_START_BLOCK
};

use chrono::Local;
use common_indexer::scraper;
use env_logger::{Builder, Target};
use price_feed::PriceFeed;
use serde::{Deserialize, Serialize};
use std::io::Write;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState{
	db_pool: SharedPool,
	price_feed: PriceFeed,
}

#[derive(Debug, Deserialize)]
struct GetEventsParams {
	block_start: u32,
	block_stop: u32,
	contract_address: Option<AccountId>,
}

async fn handle_get_trades(
	Query(params): Query<GetEventsParams>,
	app_state: AppState,
) -> impl IntoResponse {
	let conn = {
		let pool = app_state.db_pool.lock();
		pool.get().unwrap()
	};
	let result = match params.contract_address {
		Some(contract_address) =>
			get_trades_by_origin(&conn, params.block_start, params.block_stop, &contract_address),
		None => get_trades_by_range(&conn, params.block_start, params.block_stop),
	};

	match result {
		Ok(events) => {
			let multiswaps = trade_result_to_multiswaps(events);
			Json(multiswaps).into_response()
		},
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error: {}", e))
			.into_response(),
	}
}

#[derive(Debug, Serialize, Deserialize)]
struct Status {
	indexed_from: u32,
	indexed_till: u32,
}

async fn handle_get_status(app_state: AppState) -> impl IntoResponse {
	let conn = {
		let pool = app_state.db_pool.lock();
		pool.get().unwrap()
	};
	match get_indexed_till(&conn) {
		Ok(bounds) => {
			let bounds = Status { indexed_from: COMMON_START_BLOCK, indexed_till: bounds };
			Json(bounds).into_response()
		},
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error: {}", e))
			.into_response(),
	}
}

async fn handle_get_pools(app_state: AppState) -> impl IntoResponse {
	let conn = {
		let pool = app_state.db_pool.lock();
		pool.get().unwrap()
	};
	match get_pools(&conn) {
		Ok(pools) => Json(pools).into_response(),
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error: {}", e))
			.into_response(),
	}
}

#[derive(Serialize)]
struct TokenWithPrice{
	address: AccountId,
	name: Option<String>,
	symbol: Option<String>,
	decimals: u8,
	price: Option<f64>,
}

impl TokenWithPrice {
	fn new(token: Token, price_feed: &PriceFeed) -> Self {
		let price = get_price_by_token_address(&token.address, price_feed);
		TokenWithPrice {
			address: token.address,
			name: token.name,
			symbol: token.symbol,
			decimals: token.decimals,
			price,
		}
	}
}

async fn handle_get_tokens(app_state: AppState) -> impl IntoResponse {
	let conn = {
		let pool = app_state.db_pool.lock();
		pool.get().unwrap()
	};
	match get_tokens(&conn) {
		Ok(tokens) => {
			let tokens_with_prices = tokens
				.into_iter()
				.map(|token| TokenWithPrice::new(token, &app_state.price_feed))
				.collect::<Vec<TokenWithPrice>>();
			Json(tokens_with_prices).into_response()
		},
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error: {}", e))
			.into_response(),
	}
}

#[tokio::main]
async fn main() {
	Builder::new()
		.target(Target::Stdout)
		.filter(None, log::LevelFilter::Info) // Set default log level to info
		.format(|buf, record| {
			let now = Local::now();
			writeln!(
				buf,
				"{} [{}] - {}",
				now.format("%Y-%m-%d %H:%M:%S%.3f"),
				record.level(),
				record.args()
			)
		})
		.init();

	let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string());
	let rpc_azero = std::env::var("RPC_AZERO").unwrap_or_else(|_| WS_AZERO_MAINNET.to_string());
	let indexer_url = std::env::var("INDEXER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

	log::info!("Running config: port: {}, rpc_azero: {}, indexer_url: {}", port, rpc_azero, indexer_url);

	init_db().unwrap();
	tokio::spawn(async {
		let endpoints =
			Endpoints::new(rpc_azero, indexer_url);
		scraper::run(&endpoints).await;
	});

	let shared_pool = get_shared_pool();
	let price_feed = PriceFeed::new().await.unwrap();
	let app_state = AppState { db_pool: shared_pool, price_feed };

	let app = Router::new()
		.route(
			"/trades",
			get({
				let state = app_state.clone();
				move |query| handle_get_trades(query, state)
			}),
		)
		.route(
			"/status",
			get({
				let state = app_state.clone();
				move || handle_get_status(state)
			}),
		)
		.route(
			"/pools",
			get({
				let state = app_state.clone();
				move || handle_get_pools(state)
			}),
		)
		.route(
			"/tokens",
			get({
				let state = app_state.clone();
				move || handle_get_tokens(state)
			}),
		)
		.layer(CorsLayer::permissive());

	//0.0.0.0:port
	let addr = format!("0.0.0.0:{}", port);
	let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
	println!("Server running at {}", addr);
	axum::serve(listener, app.into_make_service()).await.unwrap();
}
