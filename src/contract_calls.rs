use anyhow::Context;
use ink_wrapper_types::ReadCall;

use codec::{Decode, Encode};
use pallet_contracts_primitives::ContractExecResult;
use subxt::rpc::types::Bytes;
use subxt::rpc_params;
use subxt::utils::AccountId32;

use crate::{alice_acc, azero, Client};

type Weight = azero::runtime_types::sp_weights::weight_v2::Weight;

#[derive(Encode)]
pub struct ContractCallArgs {
    /// Who is singing a tx.
    pub origin: AccountId32,
    /// Address of the contract to call.
    pub dest: AccountId32,
    /// The balance to transfer from the `origin` to `dest`.
    pub value: u128,
    /// The gas limit enforced when executing the constructor.
    pub gas_limit: Option<Weight>,
    /// The maximum amount of balance that can be charged from the caller to pay for the storage consumed.
    pub storage_deposit_limit: Option<u128>,
    /// The input data to pass to the contract.
    pub input_data: Vec<u8>,
}

async fn call_and_get(
    api: &Client,
    args: ContractCallArgs,
) -> anyhow::Result<ContractExecResult<u128>> {
    let params = rpc_params!["ContractsApi_call", Bytes(args.encode())];

    let bytes: Bytes = api.rpc().request("state_call", params).await?;
    Ok(ContractExecResult::decode(&mut bytes.as_ref())?)
}

async fn dry_run(
    api: &Client,
    origin: AccountId32,
    dest: AccountId32,
    value: u128,
    data: Vec<u8>,
) -> anyhow::Result<ContractExecResult<u128>> {
    let args = ContractCallArgs {
        origin,
        dest,
        value,
        gas_limit: None,
        input_data: data,
        storage_deposit_limit: None,
    };

    call_and_get(api, args)
        .await
        .context("RPC request error - there may be more info in node logs.")
}

pub async fn contract_read_general<T: codec::Decode + Send>(
    api: &Client,
    origin: AccountId32,
    value: u128,
    call: ReadCall<T>,
) -> anyhow::Result<T> {
    let dest_bytes: [u8; 32] = *call.account_id.as_ref();
    let dest = AccountId32::try_from(dest_bytes).unwrap();
    let result = dry_run(api, origin, dest, value, call.data)
        .await?
        .result
        .map_err(|e| anyhow::anyhow!("Contract exec failed {:?}", e))?;

    Ok(codec::Decode::decode(&mut result.data.as_slice())
        .context("Failed to decode contract call result")?)
}

pub async fn contract_read<T: codec::Decode + Send>(
    api: &Client,
    call: ReadCall<T>,
) -> anyhow::Result<T> {
    contract_read_general(api, alice_acc(), 0, call).await
}
