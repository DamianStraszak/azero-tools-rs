use codec::Encode as _;

// This file was auto-generated with ink-wrapper (https://crates.io/crates/ink-wrapper).

#[allow(dead_code)]
pub const CODE_HASH: [u8; 32] = [
    131, 184, 249, 211, 41, 15, 253, 47, 57, 129, 70, 40, 141, 30, 220, 53, 236, 74, 24, 168, 96,
    112, 178, 40, 142, 124, 174, 30, 12, 163, 166, 151,
];

#[derive(Debug, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
pub enum PSP22Error {
    Custom(String),
    InsufficientBalance(),
    InsufficientAllowance(),
    ZeroRecipientAddress(),
    ZeroSenderAddress(),
    SafeTransferCheckFailed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
pub enum NoChainExtension {}

pub mod event {
    #[allow(dead_code, clippy::large_enum_variant)]
    #[derive(Debug, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
    pub enum Event {
        Approval {
            owner: ink_primitives::AccountId,
            spender: ink_primitives::AccountId,
            amount: u128,
        },

        Transfer {
            from: Option<ink_primitives::AccountId>,
            to: Option<ink_primitives::AccountId>,
            value: u128,
        },
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Instance {
    account_id: ink_primitives::AccountId,
}

impl From<ink_primitives::AccountId> for Instance {
    fn from(account_id: ink_primitives::AccountId) -> Self {
        Self { account_id }
    }
}

impl From<Instance> for ink_primitives::AccountId {
    fn from(instance: Instance) -> Self {
        instance.account_id
    }
}

impl ink_wrapper_types::EventSource for Instance {
    type Event = event::Event;
}

#[async_trait::async_trait]
pub trait PSP22Metadata {
    fn token_name(
        &self,
    ) -> ink_wrapper_types::ReadCall<Result<Option<String>, ink_wrapper_types::InkLangError>>;
    fn token_symbol(
        &self,
    ) -> ink_wrapper_types::ReadCall<Result<Option<String>, ink_wrapper_types::InkLangError>>;
    fn token_decimals(
        &self,
    ) -> ink_wrapper_types::ReadCall<Result<u8, ink_wrapper_types::InkLangError>>;
}

#[async_trait::async_trait]
impl PSP22Metadata for Instance {
    ///  Returns the token name.
    #[allow(dead_code, clippy::too_many_arguments)]
    fn token_name(
        &self,
    ) -> ink_wrapper_types::ReadCall<Result<Option<String>, ink_wrapper_types::InkLangError>> {
        let data = vec![61, 38, 27, 212];
        ink_wrapper_types::ReadCall::new(self.account_id, data)
    }

    ///  Returns the token symbol.
    #[allow(dead_code, clippy::too_many_arguments)]
    fn token_symbol(
        &self,
    ) -> ink_wrapper_types::ReadCall<Result<Option<String>, ink_wrapper_types::InkLangError>> {
        let data = vec![52, 32, 91, 229];
        ink_wrapper_types::ReadCall::new(self.account_id, data)
    }

    ///  Returns the token decimals.
    #[allow(dead_code, clippy::too_many_arguments)]
    fn token_decimals(
        &self,
    ) -> ink_wrapper_types::ReadCall<Result<u8, ink_wrapper_types::InkLangError>> {
        let data = vec![114, 113, 183, 130];
        ink_wrapper_types::ReadCall::new(self.account_id, data)
    }
}

#[async_trait::async_trait]
pub trait PSP22 {
    fn total_supply(
        &self,
    ) -> ink_wrapper_types::ReadCall<Result<u128, ink_wrapper_types::InkLangError>>;
    fn balance_of(
        &self,
        owner: ink_primitives::AccountId,
    ) -> ink_wrapper_types::ReadCall<Result<u128, ink_wrapper_types::InkLangError>>;
    fn allowance(
        &self,
        owner: ink_primitives::AccountId,
        spender: ink_primitives::AccountId,
    ) -> ink_wrapper_types::ReadCall<Result<u128, ink_wrapper_types::InkLangError>>;
    fn transfer(
        &self,
        to: ink_primitives::AccountId,
        value: u128,
        _data: Vec<u8>,
    ) -> ink_wrapper_types::ExecCall;
    fn transfer_from(
        &self,
        from: ink_primitives::AccountId,
        to: ink_primitives::AccountId,
        value: u128,
        _data: Vec<u8>,
    ) -> ink_wrapper_types::ExecCall;
    fn approve(
        &self,
        spender: ink_primitives::AccountId,
        value: u128,
    ) -> ink_wrapper_types::ExecCall;
    fn increase_allowance(
        &self,
        spender: ink_primitives::AccountId,
        delta_value: u128,
    ) -> ink_wrapper_types::ExecCall;
    fn decrease_allowance(
        &self,
        spender: ink_primitives::AccountId,
        delta_value: u128,
    ) -> ink_wrapper_types::ExecCall;
}

#[async_trait::async_trait]
impl PSP22 for Instance {
    #[allow(dead_code, clippy::too_many_arguments)]
    fn total_supply(
        &self,
    ) -> ink_wrapper_types::ReadCall<Result<u128, ink_wrapper_types::InkLangError>> {
        let data = vec![22, 45, 248, 194];
        ink_wrapper_types::ReadCall::new(self.account_id, data)
    }

    #[allow(dead_code, clippy::too_many_arguments)]
    fn balance_of(
        &self,
        owner: ink_primitives::AccountId,
    ) -> ink_wrapper_types::ReadCall<Result<u128, ink_wrapper_types::InkLangError>> {
        let data = {
            let mut data = vec![101, 104, 56, 47];
            owner.encode_to(&mut data);
            data
        };
        ink_wrapper_types::ReadCall::new(self.account_id, data)
    }

    #[allow(dead_code, clippy::too_many_arguments)]
    fn allowance(
        &self,
        owner: ink_primitives::AccountId,
        spender: ink_primitives::AccountId,
    ) -> ink_wrapper_types::ReadCall<Result<u128, ink_wrapper_types::InkLangError>> {
        let data = {
            let mut data = vec![77, 71, 217, 33];
            owner.encode_to(&mut data);
            spender.encode_to(&mut data);
            data
        };
        ink_wrapper_types::ReadCall::new(self.account_id, data)
    }

    #[allow(dead_code, clippy::too_many_arguments)]
    fn transfer(
        &self,
        to: ink_primitives::AccountId,
        value: u128,
        _data: Vec<u8>,
    ) -> ink_wrapper_types::ExecCall {
        let data = {
            let mut data = vec![219, 32, 249, 245];
            to.encode_to(&mut data);
            value.encode_to(&mut data);
            _data.encode_to(&mut data);
            data
        };
        ink_wrapper_types::ExecCall::new(self.account_id, data)
    }

    #[allow(dead_code, clippy::too_many_arguments)]
    fn transfer_from(
        &self,
        from: ink_primitives::AccountId,
        to: ink_primitives::AccountId,
        value: u128,
        _data: Vec<u8>,
    ) -> ink_wrapper_types::ExecCall {
        let data = {
            let mut data = vec![84, 179, 199, 110];
            from.encode_to(&mut data);
            to.encode_to(&mut data);
            value.encode_to(&mut data);
            _data.encode_to(&mut data);
            data
        };
        ink_wrapper_types::ExecCall::new(self.account_id, data)
    }

    #[allow(dead_code, clippy::too_many_arguments)]
    fn approve(
        &self,
        spender: ink_primitives::AccountId,
        value: u128,
    ) -> ink_wrapper_types::ExecCall {
        let data = {
            let mut data = vec![178, 15, 27, 189];
            spender.encode_to(&mut data);
            value.encode_to(&mut data);
            data
        };
        ink_wrapper_types::ExecCall::new(self.account_id, data)
    }

    #[allow(dead_code, clippy::too_many_arguments)]
    fn increase_allowance(
        &self,
        spender: ink_primitives::AccountId,
        delta_value: u128,
    ) -> ink_wrapper_types::ExecCall {
        let data = {
            let mut data = vec![150, 214, 181, 122];
            spender.encode_to(&mut data);
            delta_value.encode_to(&mut data);
            data
        };
        ink_wrapper_types::ExecCall::new(self.account_id, data)
    }

    #[allow(dead_code, clippy::too_many_arguments)]
    fn decrease_allowance(
        &self,
        spender: ink_primitives::AccountId,
        delta_value: u128,
    ) -> ink_wrapper_types::ExecCall {
        let data = {
            let mut data = vec![254, 203, 87, 213];
            spender.encode_to(&mut data);
            delta_value.encode_to(&mut data);
            data
        };
        ink_wrapper_types::ExecCall::new(self.account_id, data)
    }
}

impl Instance {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(supply: u128) -> ink_wrapper_types::InstantiateCall<Self> {
        let data = {
            let mut data = vec![155, 174, 157, 94];
            supply.encode_to(&mut data);
            data
        };
        ink_wrapper_types::InstantiateCall::new(CODE_HASH, data)
    }
}
