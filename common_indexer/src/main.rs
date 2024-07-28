use axum::{extract::Query, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use azero_config::{AccountId, WS_AZERO_MAINNET};

use azero_universal::AccountIdSchema;
use common_indexer::{
	event_db::{
		get_indexed_till, get_pools, get_shared_pool, get_tokens, get_trades_by_origin_limited,
		get_trades_by_origin_with_limit, get_trades_by_range_limited, init_db, Pool, SharedPool,
	},
	multiswaps::{aggregate_trades, trade_result_to_multiswaps, MultiSwap},
	scraper::Endpoints,
	tokens::{get_price_by_token_address, Token},
	QueryResultMultiSwaps, COMMON_START_BLOCK,
};

use chrono::Local;
use common_indexer::scraper;
use env_logger::{Builder, Target};
use price_feed::PriceFeed;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Write};
use tower_http::cors::CorsLayer;
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
	paths(
		handle_get_status,
		handle_get_pools,
		handle_get_tokens,
		handle_get_trades,
		handle_get_volume,
		handle_get_last_week_trades,
	),
	components(schemas(
		MultiSwap,
		AccountIdSchema,
		TradeDisplay,
		TokenWithPrice,
		Status,
		Pool,
		QueryResultMultiSwaps
	))
)]
pub struct UtoipaApi;

#[derive(Clone)]
struct AppState {
	db_pool: SharedPool,
	price_feed: PriceFeed,
}

#[derive(Debug, Deserialize, IntoParams)]
struct GetTradesParams {
	block_start: u32,
	block_stop: u32,
	contract_address: Option<AccountId>,
}

#[utoipa::path(
    get,
    path = "/trades",
    responses(
        (status = 200, description = "JSON file", body = QueryResultMultiSwaps)
    ),
	params(
		GetTradesParams
	)
)]
async fn handle_get_trades(
	Query(params): Query<GetTradesParams>,
	app_state: AppState,
) -> impl IntoResponse {
	let conn = {
		let pool = app_state.db_pool.lock();
		pool.get().unwrap()
	};
	let result = match params.contract_address {
		Some(contract_address) => get_trades_by_origin_limited(
			&conn,
			params.block_start,
			params.block_stop,
			&contract_address,
		),
		None => get_trades_by_range_limited(&conn, params.block_start, params.block_stop),
	};

	match result {
		Ok(events) => {
			let multiswaps = trade_result_to_multiswaps(events);
			let query_result_multiswaps: QueryResultMultiSwaps = multiswaps.into();
			Json(query_result_multiswaps).into_response()
		},
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error: {}", e))
			.into_response(),
	}
}

#[derive(Debug, Deserialize, IntoParams)]
struct GetVolumeParams {
	account: AccountId,
}

fn get_last_week_volume(account: &AccountId, app_state: &AppState) -> anyhow::Result<f64> {
	let conn = {
		let pool = app_state.db_pool.lock();
		pool.get().unwrap()
	};
	let indexed_till = get_indexed_till(&conn)?;
	let one_week_ago = indexed_till - 7 * 24 * 60 * 60;
	let trades =
		get_trades_by_origin_with_limit(&conn, one_week_ago, indexed_till, account, None)?.data;
	let tokens = get_tokens(&conn)?;
	let mut volume = 0.0;
	let tokens = tokens
		.into_iter()
		.map(|token| ((&token.address).clone(), TokenWithPrice::new(token, &app_state.price_feed)))
		.collect::<BTreeMap<AccountId, TokenWithPrice>>();
	for trade in trades {
		let price_in = tokens[&trade.token_in].price.unwrap_or(0.0);
		let price_out = tokens[&trade.token_out].price.unwrap_or(0.0);
		let decimals_in = tokens[&trade.token_in].decimals;
		let decimals_out = tokens[&trade.token_out].decimals;
		let amount_in = trade.amount_in as f64 / 10u128.pow(decimals_in as u32) as f64;
		let amount_out = trade.amount_out as f64 / 10u128.pow(decimals_out as u32) as f64;
		let volume_in = amount_in * price_in;
		let volume_out = amount_out * price_out;
		volume += f64::max(volume_in, volume_out);
	}
	Ok(volume)
}

#[utoipa::path(
    get,
    path = "/checker/one_week_volume",
    responses(
        (status = 200, description = "JSON file", body = f64)
    ),
	params(
		GetVolumeParams
	)
)]
async fn handle_get_volume(
	Query(params): Query<GetVolumeParams>,
	app_state: AppState,
) -> impl IntoResponse {
	let result = get_last_week_volume(&params.account, &app_state);

	match result {
		Ok(volume) => Json(volume).into_response(),
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error: {}", e))
			.into_response(),
	}
}

#[derive(Debug, Serialize, ToSchema)]
struct TradeDisplay {
	block_num: u32,
	user: AccountId,
	/// Symbol of sold token.
	token_in: String,
	/// Symbol of bought token.
	token_out: String,
	amount_in: f64,
	amount_out: f64,
	/// Path of token symbols traded.
	path: Vec<String>,
	volume: f64,
}

fn get_last_week_trades(
	account: &AccountId,
	app_state: &AppState,
) -> anyhow::Result<Vec<TradeDisplay>> {
	let conn = {
		let pool = app_state.db_pool.lock();
		pool.get().unwrap()
	};
	let indexed_till = get_indexed_till(&conn)?;
	let one_week_ago = indexed_till - 7 * 24 * 60 * 60;
	let trades =
		get_trades_by_origin_with_limit(&conn, one_week_ago, indexed_till, account, None)?.data;
	let tokens = get_tokens(&conn)?;

	let tokens = tokens
		.into_iter()
		.map(|token| ((&token.address).clone(), TokenWithPrice::new(token, &app_state.price_feed)))
		.collect::<BTreeMap<AccountId, TokenWithPrice>>();
	let mut swaps = aggregate_trades(trades);
	swaps.reverse();
	swaps.truncate(500);

	let trades_display = swaps
		.into_iter()
		.map(|trade| {
			let price_in = tokens[&trade.token_in].price.unwrap_or(0.0);
			let price_out = tokens[&trade.token_out].price.unwrap_or(0.0);
			let decimals_in = tokens[&trade.token_in].decimals;
			let decimals_out = tokens[&trade.token_out].decimals;
			let amount_in = trade.amount_in as f64 / 10u128.pow(decimals_in as u32) as f64;
			let amount_out = trade.amount_out as f64 / 10u128.pow(decimals_out as u32) as f64;
			let volume_in = amount_in * price_in;
			let volume_out = amount_out * price_out;
			let volume = f64::max(volume_in, volume_out);
			let path: Vec<_> = trade
				.path
				.into_iter()
				.map(|token| tokens[&token].symbol.clone().unwrap_or("".to_string()))
				.collect();
			TradeDisplay {
				block_num: trade.block_num,
				user: trade.origin,
				token_in: path[0].clone(),
				token_out: path.last().unwrap().clone(),
				amount_in,
				amount_out,
				path,
				volume,
			}
		})
		.collect();

	Ok(trades_display)
}

#[derive(Debug, Deserialize, IntoParams)]
struct GetLastWeekTradesParams {
	account: AccountId,
}

#[utoipa::path(
    get,
    path = "/checker/one_week_trades",
    responses(
        (status = 200, description = "JSON file", body = Vec<TradeDisplay>)
    ),
	params(
		GetLastWeekTradesParams
	)
)]
async fn handle_get_last_week_trades(
	Query(params): Query<GetLastWeekTradesParams>,
	app_state: AppState,
) -> impl IntoResponse {
	let result = get_last_week_trades(&params.account, &app_state);

	match result {
		Ok(volume) => Json(volume).into_response(),
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error: {}", e))
			.into_response(),
	}
}

#[derive(Debug, Serialize, ToSchema)]
struct Status {
	indexed_from: u32,
	indexed_till: u32,
}

#[utoipa::path(
    get,
    path = "/status",
    responses(
        (status = 200, description = "JSON file", body = Status)
    ),
)]
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

#[utoipa::path(
    get,
    path = "/pools",
    responses(
        (status = 200, description = "JSON file", body = Vec<Pool>)
    ),
)]
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

#[derive(Serialize, ToSchema)]
struct TokenWithPrice {
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

#[utoipa::path(
    get,
    path = "/tokens",
    responses(
        (status = 200, description = "JSON file", body = Vec<TokenWithPrice>)
    ),
)]
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
	let indexer_url = std::env::var("INDEXER_URL")
		.unwrap_or_else(|_| "https://indexer.azero-tools.com".to_string());

	log::info!(
		"Running config: port: {}, rpc_azero: {}, indexer_url: {}",
		port,
		rpc_azero,
		indexer_url
	);

	init_db().unwrap();
	tokio::spawn(async {
		let endpoints = Endpoints::new(rpc_azero, indexer_url);
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
		.route(
			"/checker/one_week_volume",
			get({
				let state = app_state.clone();
				move |query| handle_get_volume(query, state)
			}),
		)
		.route(
			"/checker/one_week_trades",
			get({
				let state = app_state.clone();
				move |query| handle_get_last_week_trades(query, state)
			}),
		)
		.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", UtoipaApi::openapi()))
		.layer(CorsLayer::permissive());

	let addr = format!("0.0.0.0:{}", port);
	let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
	log::info!("Server running at {}", addr);
	axum::serve(listener, app.into_make_service()).await.unwrap();
}
