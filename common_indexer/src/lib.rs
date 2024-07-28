use multiswaps::MultiSwap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::SerializeAs;
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

pub fn u128_dec() -> utoipa::openapi::schema::Schema {
	utoipa::openapi::ObjectBuilder::new()
		.description(Some("A 128-bit unsigned integer encoded as a string in decimal"))
		.example(Some(Value::String("\"1234567891011121314\"".into())))
		.into()
}
