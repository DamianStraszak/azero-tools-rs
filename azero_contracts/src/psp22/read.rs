use super::psp22_wrapper::{self, PSP22Metadata, PSP22};
use crate::read::{read_from_contract, ReadFor};
use azero_config::{AccountId, BlockHash, RpcClient};

pub async fn read_total_supply(
	api: &RpcClient,
	contract_address: &AccountId,
	at: Option<BlockHash>,
) -> ReadFor<u128> {
	let instance: psp22_wrapper::Instance =
		ink_primitives::AccountId::try_from(contract_address.as_ref()).unwrap().into();
	read_from_contract(api, instance.total_supply(), at).await
}

pub async fn read_decimals(
	api: &RpcClient,
	contract_address: &AccountId,
	at: Option<BlockHash>,
) -> ReadFor<u8> {
	let instance: psp22_wrapper::Instance =
		ink_primitives::AccountId::try_from(contract_address.as_ref()).unwrap().into();
	read_from_contract(api, instance.token_decimals(), at).await
}

pub async fn read_name(
	api: &RpcClient,
	contract_address: &AccountId,
	at: Option<BlockHash>,
) -> ReadFor<Option<String>> {
	let instance: psp22_wrapper::Instance =
		ink_primitives::AccountId::try_from(contract_address.as_ref()).unwrap().into();
	read_from_contract(api, instance.token_name(), at).await
}

pub async fn read_symbol(
	api: &RpcClient,
	contract_address: &AccountId,
	at: Option<BlockHash>,
) -> ReadFor<Option<String>> {
	let instance: psp22_wrapper::Instance =
		ink_primitives::AccountId::try_from(contract_address.as_ref()).unwrap().into();
	read_from_contract(api, instance.token_symbol(), at).await
}

pub async fn read_balance_of(
	api: &RpcClient,
	contract_address: &AccountId,
	user: &AccountId,
	at: Option<BlockHash>,
) -> ReadFor<u128> {
	let instance: psp22_wrapper::Instance =
		ink_primitives::AccountId::try_from(contract_address.as_ref()).unwrap().into();
	let user = ink_primitives::AccountId::try_from(user.as_ref()).unwrap().into();
	read_from_contract(api, instance.balance_of(user), at).await
}
