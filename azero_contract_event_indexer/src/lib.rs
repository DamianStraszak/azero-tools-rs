use azero_config::BlockHeader;

pub type Client = azero_config::Client;
pub type RpcClient = azero_config::RpcClient;
pub type BlockHash = azero_config::BlockHash;
pub type AccountId = azero_config::AccountId;

pub mod event_db;
pub mod scraper;

//const ENDPOINTS : [&str; 3] = ["wss://aleph-zero-rpc.dwellir.com",
// "wss://aleph-zero.api.onfinality.io/public-ws", "wss://ws.azero.dev"];
const ENDPOINTS: [&str; 1] = ["wss://ws.azero.dev"]; //"wss://ws.azero.dev", ,

pub fn random_endpoint() -> &'static str {
	let index = rand::random::<usize>() % ENDPOINTS.len();
	ENDPOINTS[index]
}

pub async fn get_rpc_client() -> anyhow::Result<RpcClient> {
	let endpoint = random_endpoint();
	RpcClient::from_url(endpoint)
		.await
		.map_err(|e| anyhow::anyhow!("Failed to connect to endpoint {}: {}", endpoint, e))
}

pub async fn get_client() -> anyhow::Result<Client> {
	let endpoint = random_endpoint();
	Client::from_url(endpoint)
		.await
		.map_err(|e| anyhow::anyhow!("Failed to connect to endpoint {}: {}", endpoint, e))
}

pub async fn get_current_best_finalized_header(client: &Client) -> anyhow::Result<BlockHeader> {
	let current_block = client.blocks().at_latest().await?;
	Ok(current_block.header().clone())
}

async fn get_finalized_block_num() -> anyhow::Result<u32> {
	let client = get_client().await?;
	let header = get_current_best_finalized_header(&client).await?;
	Ok(header.number)
}

pub async fn start_indexer() -> ! {
	let current_num = loop {
		match get_finalized_block_num().await {
			Ok(num) => break num,
			Err(e) => {
				log::error!("Error getting finalized block number: {}", e);
				tokio::time::sleep(std::time::Duration::from_secs(2)).await;
			},
		}
	};
	event_db::init_db(current_num).unwrap();
	scraper::scrape().await
}
