use azero_config::{alice_acc, AccountId, Client};
use codec::{Decode, Encode};
use ink_wrapper_types::{InkLangError, ReadCall};
use pallet_contracts_primitives::ContractExecResult;
use sp_runtime::DispatchError;
use subxt::{rpc::types::Bytes, rpc_params};

type Weight = azero_runtime_types::v_69::runtime_types::sp_weights::weight_v2::Weight;

#[derive(Encode)]
pub struct ContractCallArgs {
	/// Who is singing a tx.
	pub origin: AccountId,
	/// Address of the contract to call.
	pub dest: AccountId,
	/// The balance to transfer from the `origin` to `dest`.
	pub value: u128,
	/// The gas limit enforced when executing the constructor.
	pub gas_limit: Option<Weight>,
	/// The maximum amount of balance that can be charged from the caller to pay for the storage
	/// consumed.
	pub storage_deposit_limit: Option<u128>,
	/// The input data to pass to the contract.
	pub input_data: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum ContractReadError {
	#[error("Dispatch error {:?}", "{0}")]
	Dispatch(DispatchError),
	#[error("Rpc decode error {0}")]
	ResultDecode(#[from] codec::Error),
	#[error("InkLang error {0}")]
	InkLang(#[from] InkLangError),
}

#[derive(Debug, thiserror::Error)]
pub enum RpcCallError {
	#[error("Rpc request failed {0}")]
	Rpc(#[from] subxt::Error),
	#[error("Rpc decode error {0}")]
	RpcDecode(#[from] codec::Error),
}

async fn call_and_get(
	api: &Client,
	args: ContractCallArgs,
) -> Result<ContractExecResult<u128>, RpcCallError> {
	let params = rpc_params!["ContractsApi_call", Bytes(args.encode())];
	let bytes: Bytes = api.rpc().request("state_call", params).await?;
	Ok(ContractExecResult::decode(&mut bytes.as_ref())?)
}

async fn dry_run(
	api: &Client,
	origin: AccountId,
	dest: AccountId,
	value: u128,
	data: Vec<u8>,
) -> Result<ContractExecResult<u128>, RpcCallError> {
	let args = ContractCallArgs {
		origin,
		dest,
		value,
		gas_limit: None,
		input_data: data,
		storage_deposit_limit: None,
	};

	call_and_get(api, args).await
}

pub async fn contract_read_general<T: codec::Decode + Send>(
	api: &Client,
	origin: AccountId,
	value: u128,
	call: ReadCall<T>,
) -> Result<Result<T, ContractReadError>, RpcCallError> {
	let dest_bytes: [u8; 32] = *call.account_id.as_ref();
	let dest = AccountId::from(dest_bytes);
	let rpc_result = dry_run(api, origin, dest, value, call.data).await?.result;
	let result = match rpc_result {
		Err(e) => Err(ContractReadError::Dispatch(e)),
		Ok(exec_return) => match codec::Decode::decode(&mut exec_return.data.as_slice()) {
			Ok(v) => Ok(v),
			Err(e) => Err(ContractReadError::ResultDecode(e)),
		},
	};
	Ok(result)
}

pub async fn contract_read<T: codec::Decode + Send>(
	api: &Client,
	call: ReadCall<T>,
) -> Result<Result<T, ContractReadError>, RpcCallError> {
	contract_read_general(api, alice_acc(), 0, call).await
}

pub type ReadFor<T> = Result<Result<T, ContractReadError>, RpcCallError>;

pub async fn read_from_contract<T: codec::Decode + Send + Sync>(
	api: &Client,
	call: ReadCall<Result<T, InkLangError>>,
) -> Result<Result<T, ContractReadError>, RpcCallError> {
	let read_result = contract_read(api, call).await?;
	let res = match read_result {
		Ok(Ok(v)) => Ok(v),
		Ok(Err(e)) => Err(e.into()),
		Err(e) => Err(e),
	};
	Ok(res)
}
