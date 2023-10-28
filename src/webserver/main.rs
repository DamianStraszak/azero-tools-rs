use askama::Template;
use axum::{
    extract::Path, extract::Query, http::StatusCode, response::Redirect, routing::get, Router,
};
use azero_tools_rs::token_db::{tracker::TokenDBTracker, TokenDB};
use serde::Deserialize;
use std::env;
use std::net::SocketAddr;
use std::str::FromStr;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let static_files_path = env::var("STATIC_FILES_PATH").unwrap_or("./static/".to_string());
    let static_files_service = ServeDir::new(static_files_path);
    let token_db = TokenDB::from_disk();
    let tracker = TokenDBTracker::new(token_db.clone()).await.unwrap();
    tokio::spawn(async move { tracker.run().await });
    let app = Router::new()
        .route("/", get(handler))
        .route("/account/:account_id", get(account_handler))
        .route("/search", get(search_account))
        .nest_service("/static", static_files_service)
        .layer(axum::extract::Extension(token_db));

    let addr = SocketAddr::from_str("127.0.0.1:3000").unwrap();
    println!("Server running at http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler(
    axum::extract::Extension(token_db): axum::extract::Extension<TokenDB>,
) -> impl axum::response::IntoResponse {
    let db_summary = token_db.get_summary();
    axum::response::Html(db_summary.render().unwrap())
}

async fn account_handler(
    Path(account_id): Path<String>,
    axum::extract::Extension(token_db): axum::extract::Extension<TokenDB>,
) -> impl axum::response::IntoResponse {
    let account_details = token_db.get_account_details(account_id); // Implement this function to fetch the account details
    axum::response::Html(account_details.render().unwrap())
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    acc: Option<String>,
}

async fn search_account(Query(params): Query<SearchQuery>) -> Result<Redirect, StatusCode> {
    match params.acc {
        Some(acc) => {
            let acc = acc.trim();
            if !acc.is_empty() {
                let uri = format!("/account/{}", acc);
                Ok(Redirect::to(&uri))
            } else {
                Ok(Redirect::to("/"))
            }
        }
        None => Ok(Redirect::to("/")),
    }
}
