use std::collections::BTreeMap;
use azero_config::AccountId;
use crate::storage::ContractStorage;

pub fn storage_to_balances(storage: &ContractStorage) -> BTreeMap<AccountId, u128> {
	let magic_prefixes: Vec<Vec<u8>> = ["3b8d451d", "e4aae541", "264866c2", "d446c745"]
		.iter()
		.map(|s| hex::decode(s).unwrap())
		.collect();

	let storage_36_16: BTreeMap<Vec<u8>, Vec<u8>> = storage
		.iter()
		.filter(|(k, v)| k.len() == 36 && v.len() == 16)
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();
	let prefixes: Vec<Vec<u8>> = storage_36_16.keys().map(|k| k[..4].to_vec()).collect();
	if prefixes.is_empty() {
		return BTreeMap::new();
	}
	for magic_prefix in magic_prefixes {
		if prefixes.contains(&magic_prefix.to_owned()) {
			let mut balances = BTreeMap::new();
			for (k, v) in storage_36_16.iter() {
				if k.starts_with(&magic_prefix) {
					let array_u8: [u8; 32] = k[4..].try_into().unwrap();
					let account = AccountId::from(array_u8);
					let balance = codec::Decode::decode(&mut &v[..]).unwrap();
					if balance > 0 {
						balances.insert(account, balance);
					}
				}
			}
			return balances;
		}
	}
	BTreeMap::new()
}