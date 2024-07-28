use azero_config::{alice_acc, AccountId, BlockHash, RpcClient};
use codec::{Decode, Encode};
use ink_wrapper_types::{InkLangError, ReadCall};
use pallet_contracts_primitives::ContractExecResult;
use sp_runtime::DispatchError;
use subxt::{backend::legacy::rpc_methods::Bytes, rpc_params};

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
	api: &RpcClient,
	args: ContractCallArgs,
	at: Option<BlockHash>,
) -> Result<ContractExecResult<u128>, RpcCallError> {
	let api: subxt::backend::rpc::RpcClient = api.clone().into();
	let params = rpc_params!["ContractsApi_call", Bytes(args.encode()), at];
	let bytes: Bytes = api.request("state_call", params).await?;
	Ok(ContractExecResult::decode(&mut bytes.as_ref())?)
}

async fn dry_run(
	api: &RpcClient,
	origin: AccountId,
	dest: AccountId,
	value: u128,
	data: Vec<u8>,
	at: Option<BlockHash>,
) -> Result<ContractExecResult<u128>, RpcCallError> {
	let args = ContractCallArgs {
		origin,
		dest,
		value,
		gas_limit: None,
		input_data: data,
		storage_deposit_limit: None,
	};

	call_and_get(api, args, at).await
}

pub async fn contract_read_general<T: codec::Decode + Send>(
	api: &RpcClient,
	origin: AccountId,
	value: u128,
	call: ReadCall<T>,
	at: Option<BlockHash>,
) -> Result<Result<T, ContractReadError>, RpcCallError> {
	let dest_bytes: [u8; 32] = *call.account_id.as_ref();
	let dest = AccountId::from(dest_bytes);
	let rpc_result = dry_run(api, origin, dest, value, call.data, at).await?.result;
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
	api: &RpcClient,
	call: ReadCall<T>,
	caller: AccountId,
	at: Option<BlockHash>,
) -> Result<Result<T, ContractReadError>, RpcCallError> {
	contract_read_general(api, caller, 0, call, at).await
}

pub type ReadFor<T> = Result<Result<T, ContractReadError>, RpcCallError>;

pub async fn read_from_contract<T: codec::Decode + Send + Sync>(
	api: &RpcClient,
	call: ReadCall<Result<T, InkLangError>>,
	at: Option<BlockHash>,
) -> ReadFor<T> {
	read_from_contract_custom_caller(api, call, alice_acc(), at).await
}

pub async fn read_from_contract_custom_caller<T: codec::Decode + Send + Sync>(
	api: &RpcClient,
	call: ReadCall<Result<T, InkLangError>>,
	caller: AccountId,
	at: Option<BlockHash>,
) -> ReadFor<T> {
	let read_result = contract_read(api, call, caller, at).await?;
	let res = match read_result {
		Ok(Ok(v)) => Ok(v),
		Ok(Err(e)) => Err(e.into()),
		Err(e) => Err(e),
	};
	Ok(res)
}
