use multiswaps::MultiSwap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::SerializeAs;
use std::str::FromStr;
use utoipa::ToSchema;

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

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct QueryResult<T> {
	pub data: T,
	/// Whether the result is complete or it is a partial result (full result didn't fit within the
	/// limit)
	pub is_complete: bool,
}

// This type is separate because of issues with utoipa and generating a Schema with generics
#[derive(Serialize, Clone, Debug, ToSchema)]
pub struct QueryResultMultiSwaps {
	pub data: Vec<MultiSwap>,
	/// Whether the result is complete or it is a partial result (full result didn't fit within the
	/// limit)
	pub is_complete: bool,
}

impl From<QueryResult<Vec<MultiSwap>>> for QueryResultMultiSwaps {
	fn from(result_trades: QueryResult<Vec<MultiSwap>>) -> QueryResultMultiSwaps {
		QueryResultMultiSwaps { data: result_trades.data, is_complete: result_trades.is_complete }
	}
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

pub fn u128_dec() -> utoipa::openapi::schema::Schema {
	utoipa::openapi::ObjectBuilder::new()
		.description(Some("A 128-bit unsigned integer encoded as a string in decimal"))
		.example(Some(Value::String("\"1234567891011121314\"".into())))
		.into()
}

const PSP_LIST: [&str; 11] = [
	"5CtuFVgEUz13SFPVY6s2cZrnLDEkxQXc19aXrNARwEBeCXgg",
	"5EA7h2xCP9TkAwEQ8Km2b7aQChPKVCqcS2BJWqYavoXiEsfx",
	"5EEtCdKLyyhQnNQWWWPM1fMDx1WdVuiaoR9cA6CWttgyxtuJ",
	"5ESKJbkpVa1ppUCmrkCmaZDHqm9SHihws9Uqqsoi4VrDCDLE",
	"5EoFQd36196Duo6fPTz2MWHXRzwTJcyETHyCyaB3rb61Xo2u",
	"5Et3dDcXUiThrBCot7g65k3oDSicGy4qC82cq9f911izKNtE",
	"5F9aiiwLMPC6fFxxwqHvJpm7h5T4Xm93mJT6cpDrQnKkLFoK",
	"5FYFojNCJVFR2bBNKfAePZCa72ZcVX5yeTv8K9bzeUo8D83Z",
	"5GCubYQbm9x6TQbthbWpUVrgEibXMDXhgisw8DFYCpPJQ5f7",
	"5GVjxVdUMr5dQX9TSvvwWq42jyRaXLN65MDh4A8jhdG4Rz1A",
	"5HZxA385SYeydqZUpTeKj7D37T1bL9N6JA7Xde5QMP8qiSym",
];

pub fn psp_list() -> Vec<AccountId> {
	PSP_LIST.iter().map(|x| AccountId::from_str(x).unwrap()).collect()
}
