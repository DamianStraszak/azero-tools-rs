use crate::{contract_calls::contract_read, Client};
use psp22_wrapper::{PSP22Metadata, PSP22};
use subxt::utils::AccountId32;
mod psp22_wrapper;

pub async fn read_total_supply(api: &Client, contract_address: AccountId32) -> Option<u128> {
    let account_id = ink_primitives::AccountId::try_from(contract_address.as_ref()).unwrap();
    let instance: psp22_wrapper::Instance = account_id.into();
    contract_read(api, instance.total_supply())
        .await
        .unwrap()
        .ok()
}

pub async fn read_decimals(api: &Client, contract_address: AccountId32) -> Option<u8> {
    let account_id = ink_primitives::AccountId::try_from(contract_address.as_ref()).unwrap();
    let instance: psp22_wrapper::Instance = account_id.into();
    contract_read(api, instance.token_decimals())
        .await
        .unwrap()
        .ok()
}
