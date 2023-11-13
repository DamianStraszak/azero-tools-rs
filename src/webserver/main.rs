use askama::Template;
use axum::{
    extract::Path, extract::Query, http::StatusCode, response::Redirect, routing::get, Router,
};
use azero_tools_rs::token_db::{tracker::TokenDBTracker, TokenDB};
use azero_tools_rs::{
    MAINNET_TOKEN_DB_FILEPATH_JSON, TESTNET_TOKEN_DB_FILEPATH_JSON, WS_AZERO_MAINNET,
    WS_AZERO_TESTNET,
};
use serde::Deserialize;
use std::env;
use std::net::SocketAddr;
use std::str::FromStr;
use tower_http::services::ServeDir;

#[derive(Clone)]
struct NetworkTokens {
    network: String,
    db: TokenDB,
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    let static_files_path = env::var("STATIC_FILES_PATH").unwrap_or("./static/".to_string());
    let static_files_service = ServeDir::new(static_files_path);
    let testnet_app = create_app("testnet", TESTNET_TOKEN_DB_FILEPATH_JSON, WS_AZERO_TESTNET).await;
    let mainnet_app = create_app("mainnet", MAINNET_TOKEN_DB_FILEPATH_JSON, WS_AZERO_MAINNET).await;
    let app = Router::new()
        .nest("/testnet", testnet_app)
        .nest("/mainnet", mainnet_app)
        .nest_service("/static", static_files_service)
        .route("/", get(Redirect::to("/mainnet")));

    let addr = SocketAddr::from_str("127.0.0.1:3000").unwrap();
    println!("Server running at http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn create_app(network: &str, backup_path: &str, ws_url: &str) -> Router {
    let token_db = TokenDB::from_disk(backup_path);
    let tracker = TokenDBTracker::new(token_db.clone(), network, backup_path, ws_url)
        .await
        .unwrap();
    let state = NetworkTokens {
        network: network.to_string(),
        db: token_db.clone(),
    };
    tokio::spawn(async move { tracker.run().await });

    Router::new()
        .route("/", get(handler))
        .route("/account/:account_id", get(account_handler))
        .route("/search", get(search_account))
        .layer(axum::extract::Extension(state))
}

async fn handler(
    axum::extract::Extension(state): axum::extract::Extension<NetworkTokens>,
) -> impl axum::response::IntoResponse {
    let network = state.network.clone();
    let db_summary = state.db.get_summary(network);
    axum::response::Html(db_summary.render().unwrap())
}

async fn account_handler(
    Path(account_id): Path<String>,
    axum::extract::Extension(state): axum::extract::Extension<NetworkTokens>,
) -> impl axum::response::IntoResponse {
    let network = state.network.clone();
    let account_details = state.db.get_account_details(network, account_id);
    axum::response::Html(account_details.render().unwrap())
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    acc: Option<String>,
}

async fn search_account(
    Query(params): Query<SearchQuery>,
    axum::extract::Extension(state): axum::extract::Extension<NetworkTokens>,
) -> Result<Redirect, StatusCode> {
    let network = state.network.clone();
    match params.acc {
        Some(acc) => {
            let acc = acc.trim();
            if !acc.is_empty() {
                let uri = format!("/{}/account/{}", network, acc);
                Ok(Redirect::to(&uri))
            } else {
                Ok(Redirect::to(&format!("/{}", network)))
            }
        }
        None => Ok(Redirect::to(&format!("/{}", network))),
    }
}
