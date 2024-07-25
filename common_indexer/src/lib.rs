use serde::{Deserialize, Serialize};
use serde_with::SerializeAs;

pub type Client = azero_config::Client;
pub type RpcClient = azero_config::RpcClient;
pub type BlockHash = azero_config::BlockHash;
pub type AccountId = azero_config::AccountId;

pub mod event_db;
pub mod multiswaps;
pub mod pools;
pub mod scraper;
pub mod tokens;

pub const COMMON_START_BLOCK: u32 = 78272779;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QueryResult<T> {
	pub data: T,
	pub is_complete: bool,
}

pub struct U128AsDecString;

impl SerializeAs<u128> for U128AsDecString {
	fn serialize_as<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		value.to_string().serialize(serializer)
	}
}
