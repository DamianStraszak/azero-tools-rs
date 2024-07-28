use azero_config::{BlockHash, Client, Config, RpcClient};
use subxt::backend::legacy::LegacyRpcMethods;

pub mod contract_events;
pub mod contract_info;

pub async fn get_hash_from_number(
	client: &RpcClient,
	num: u32,
) -> anyhow::Result<Option<BlockHash>> {
	let rpc_methods = LegacyRpcMethods::<Config>::new(client.clone());
	let n = subxt::backend::legacy::rpc_methods::NumberOrHex::Number(num as u64);
	let block_hash = rpc_methods.chain_get_block_hash(Some(n)).await?;
	Ok(block_hash)
}

pub async fn initialize_client(url: &str) -> (RpcClient, Client) {
	loop {
		match RpcClient::from_url(url).await {
			Ok(rpc_client) => match Client::from_rpc_client(rpc_client.clone()).await {
				Ok(client) => {
					return (rpc_client, client);
				},
				Err(e) => {
					log::info!("Error {} initializing client at {}", e, url);
					tokio::time::sleep(std::time::Duration::from_secs(2)).await;
				},
			},
			Err(e) => {
				log::info!("Error {} initializing client at {}", e, url);
				tokio::time::sleep(std::time::Duration::from_secs(2)).await;
			},
		}
	}
}

pub struct AccountIdSchema;

impl<'s> utoipa::ToSchema<'s> for AccountIdSchema {
	fn schema() -> (&'s str, utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>) {
		(
			"AccountId",
			utoipa::openapi::ObjectBuilder::new()
				.example(Some("\"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY\"".into()))
				.description(Some("ss58 encoded AccountId"))
				.into(),
		)
	}
}
