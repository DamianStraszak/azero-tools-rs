pub type Client = azero_config::Client;
pub type BlockHash = azero_config::BlockHash;
pub type AccountId = azero_config::AccountId;

pub mod token_db;

pub const MAINNET_TOKEN_DB_FILEPATH_JSON: &str = "mainnet_token_db.json";
pub const TESTNET_TOKEN_DB_FILEPATH_JSON: &str = "testnet_token_db.json";

pub async fn initialize_client(url: &str) -> Client {
	loop {
		match Client::from_url(url).await {
			Ok(client) => break client,
			Err(e) => {
				println!("Error {} initializing client at {}", e, url);
				tokio::time::sleep(std::time::Duration::from_secs(2)).await;
			},
		}
	}
}
