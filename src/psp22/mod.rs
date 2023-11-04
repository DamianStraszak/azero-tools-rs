use crate::{
    contracts::{contract_read, ContractReadError, RpcCallError},
    Client,
};
use anyhow::Result;
use ink_wrapper_types::{InkLangError, ReadCall};
use psp22_wrapper::{PSP22Metadata, PSP22};
use subxt::utils::AccountId32;
mod psp22_wrapper;

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

pub async fn read_total_supply(api: &Client, contract_address: &AccountId32) -> ReadFor<u128> {
    let instance: psp22_wrapper::Instance =
        ink_primitives::AccountId::try_from(contract_address.as_ref())
            .unwrap()
            .into();
    read_from_contract(api, instance.total_supply()).await
}

pub async fn read_decimals(api: &Client, contract_address: &AccountId32) -> ReadFor<u8> {
    let instance: psp22_wrapper::Instance =
        ink_primitives::AccountId::try_from(contract_address.as_ref())
            .unwrap()
            .into();
    read_from_contract(api, instance.token_decimals()).await
}

pub async fn read_name(api: &Client, contract_address: &AccountId32) -> ReadFor<Option<String>> {
    let instance: psp22_wrapper::Instance =
        ink_primitives::AccountId::try_from(contract_address.as_ref())
            .unwrap()
            .into();
    read_from_contract(api, instance.token_name()).await
}

pub async fn read_symbol(api: &Client, contract_address: &AccountId32) -> ReadFor<Option<String>> {
    let instance: psp22_wrapper::Instance =
        ink_primitives::AccountId::try_from(contract_address.as_ref())
            .unwrap()
            .into();
    read_from_contract(api, instance.token_symbol()).await
}
