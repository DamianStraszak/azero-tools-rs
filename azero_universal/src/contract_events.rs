use azero_config::{AccountId, Config};
use serde::{Deserialize, Serialize};
use subxt::{events::EventDetails, utils::H256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenericContractEvent {
	Instantiated { deployer: AccountId, contract: AccountId },
	Terminated { contract: AccountId, beneficiary: AccountId },
	CodeStored { code_hash: H256 },
	ContractEmitted { contract: AccountId, data: Vec<u8> },
	CodeRemoved { code_hash: H256 },
	ContractCodeUpdated { contract: AccountId, new_code_hash: H256, old_code_hash: H256 },
	Called { caller: Origin, contract: AccountId },
	DelegateCalled { contract: AccountId, code_hash: H256 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin {
	Root,
	Signed(AccountId),
}

impl From<AccountId> for Origin {
	fn from(account: AccountId) -> Self {
		Self::Signed(account)
	}
}

mod v_69 {
	use super::{GenericContractEvent, Origin};
	use azero_config::Config;
	use azero_runtime_types::v_69::{
		runtime_types::pallet_contracts::{pallet::Event as ContractsEvent, Origin as CallOrigin},
		Event,
	};
	use subxt::events::EventDetails;

	pub(crate) fn into_contract_event(
		event: &EventDetails<Config>,
	) -> Option<GenericContractEvent> {
		if let Ok(ev) = event.as_root_event::<Event>() {
			let maybe_event = match ev {
				Event::Contracts(contracts_event) => match contracts_event {
					ContractsEvent::Instantiated { deployer, contract } =>
						Some(GenericContractEvent::Instantiated { deployer, contract }),

					ContractsEvent::Terminated { contract, beneficiary } =>
						Some(GenericContractEvent::Terminated { contract, beneficiary }),
					ContractsEvent::CodeStored { code_hash, deposit_held: _, uploader: _ } =>
						Some(GenericContractEvent::CodeStored { code_hash }),

					ContractsEvent::ContractEmitted { contract, data } =>
						Some(GenericContractEvent::ContractEmitted { contract, data }),

					ContractsEvent::CodeRemoved { code_hash, deposit_released: _, remover: _ } =>
						Some(GenericContractEvent::CodeRemoved { code_hash }),

					ContractsEvent::ContractCodeUpdated {
						contract,
						new_code_hash,
						old_code_hash,
					} => Some(GenericContractEvent::ContractCodeUpdated {
						contract,
						new_code_hash,
						old_code_hash,
					}),
					ContractsEvent::Called { caller, contract } => match caller {
						CallOrigin::Signed(c) => Some(GenericContractEvent::Called {
							caller: Origin::Signed(c),
							contract,
						}),
						CallOrigin::Root =>
							Some(GenericContractEvent::Called { caller: Origin::Root, contract }),
						CallOrigin::__Ignore(_) => None,
					},

					ContractsEvent::DelegateCalled { contract, code_hash } =>
						Some(GenericContractEvent::DelegateCalled { contract, code_hash }),
					ContractsEvent::StorageDepositTransferredAndHeld { .. } => None,
					ContractsEvent::StorageDepositTransferredAndReleased { .. } => None,
				},
				_ => None,
			};
			if let Some(event) = maybe_event {
				return Some(event);
			}
		}
		None
	}
}

mod v_73 {
	use super::{GenericContractEvent, Origin};
	use azero_config::Config;
	use azero_runtime_types::v_73::{
		runtime_types::pallet_contracts::{pallet::Event as ContractsEvent, Origin as CallOrigin},
		Event,
	};
	use subxt::events::EventDetails;

	pub(crate) fn into_contract_event(
		event: &EventDetails<Config>,
	) -> Option<GenericContractEvent> {
		if let Ok(ev) = event.as_root_event::<Event>() {
			let maybe_event = match ev {
				Event::Contracts(contracts_event) => match contracts_event {
					ContractsEvent::Instantiated { deployer, contract } =>
						Some(GenericContractEvent::Instantiated { deployer, contract }),

					ContractsEvent::Terminated { contract, beneficiary } =>
						Some(GenericContractEvent::Terminated { contract, beneficiary }),
					ContractsEvent::CodeStored { code_hash, deposit_held: _, uploader: _ } =>
						Some(GenericContractEvent::CodeStored { code_hash }),

					ContractsEvent::ContractEmitted { contract, data } =>
						Some(GenericContractEvent::ContractEmitted { contract, data }),

					ContractsEvent::CodeRemoved { code_hash, deposit_released: _, remover: _ } =>
						Some(GenericContractEvent::CodeRemoved { code_hash }),

					ContractsEvent::ContractCodeUpdated {
						contract,
						new_code_hash,
						old_code_hash,
					} => Some(GenericContractEvent::ContractCodeUpdated {
						contract,
						new_code_hash,
						old_code_hash,
					}),
					ContractsEvent::Called { caller, contract } => match caller {
						CallOrigin::Signed(c) => Some(GenericContractEvent::Called {
							caller: Origin::Signed(c),
							contract,
						}),
						CallOrigin::Root =>
							Some(GenericContractEvent::Called { caller: Origin::Root, contract }),
						CallOrigin::__Ignore(_) => None,
					},

					ContractsEvent::DelegateCalled { contract, code_hash } =>
						Some(GenericContractEvent::DelegateCalled { contract, code_hash }),
					ContractsEvent::StorageDepositTransferredAndHeld { .. } => None,
					ContractsEvent::StorageDepositTransferredAndReleased { .. } => None,
				},
				_ => None,
			};
			if let Some(event) = maybe_event {
				return Some(event);
			}
		}
		None
	}
}

pub fn backwards_compatible_into_contract_event(
	event: EventDetails<Config>,
) -> Option<GenericContractEvent> {
	if let Some(event) = v_73::into_contract_event(&event) {
		return Some(event);
	}

	if let Some(event) = v_69::into_contract_event(&event) {
		return Some(event);
	}

	None
}
