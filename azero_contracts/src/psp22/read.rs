use super::psp22_wrapper::{self, PSP22Metadata, PSP22};
use crate::read::{read_from_contract, ReadFor};
use azero_config::{AccountId, Client};

pub async fn read_total_supply(api: &Client, contract_address: &AccountId) -> ReadFor<u128> {
	let instance: psp22_wrapper::Instance =
		ink_primitives::AccountId::try_from(contract_address.as_ref()).unwrap().into();
	read_from_contract(api, instance.total_supply()).await
}

pub async fn read_decimals(api: &Client, contract_address: &AccountId) -> ReadFor<u8> {
	let instance: psp22_wrapper::Instance =
		ink_primitives::AccountId::try_from(contract_address.as_ref()).unwrap().into();
	read_from_contract(api, instance.token_decimals()).await
}

pub async fn read_name(api: &Client, contract_address: &AccountId) -> ReadFor<Option<String>> {
	let instance: psp22_wrapper::Instance =
		ink_primitives::AccountId::try_from(contract_address.as_ref()).unwrap().into();
	read_from_contract(api, instance.token_name()).await
}

pub async fn read_symbol(api: &Client, contract_address: &AccountId) -> ReadFor<Option<String>> {
	let instance: psp22_wrapper::Instance =
		ink_primitives::AccountId::try_from(contract_address.as_ref()).unwrap().into();
	read_from_contract(api, instance.token_symbol()).await
}
