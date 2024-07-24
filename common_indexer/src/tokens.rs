use azero_config::RpcClient;
use price_feed::PriceFeed;
use serde::Serialize;

use crate::AccountId;
use anyhow::Result;
use std::{collections::BTreeMap, str::FromStr};

pub const ONE_AZERO: u128 = 1_000_000_000_000;

#[derive(Debug, Clone, Serialize)]
pub struct Token {
	pub address: AccountId,
	pub name: Option<String>,
	pub symbol: Option<String>,
	pub decimals: u8,
}

pub fn to_human(amount: u128, decimals: u8) -> f64 {
	amount as f64 / 10u128.pow(decimals as u32) as f64
}

pub fn from_human(amount: f64, decimals: u8) -> u128 {
	(amount * 10u128.pow(decimals as u32) as f64) as u128
}

pub fn native_to_human(amount: u128) -> f64 {
	to_human(amount, 12)
}

pub fn native_from_human(amount: f64) -> u128 {
	from_human(amount, 12)
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
	pub tokens: BTreeMap<AccountId, Token>,
}

impl TokenInfo {
	pub fn new() -> Self {
		TokenInfo { tokens: BTreeMap::new() }
	}

	pub fn add_token(&mut self, address: AccountId, symbol: Option<String>, name: Option<String>, decimals: u8) {
		self.tokens.insert(address.clone(), Token { symbol, name, decimals, address });
	}

	pub fn get_token_maybe(&self, address: &AccountId) -> Option<&Token> {
		self.tokens.get(address)
	}

	pub fn get_token(&self, address: &AccountId) -> Token {
		self.tokens.get(address).unwrap().clone()
	}

	pub fn tokens(&self) -> Vec<Token> {
		self.tokens.values().cloned().collect()
	}
}

pub async fn get_token_info(
	rpc_client: &RpcClient,
	addresses: Vec<AccountId>,
) -> Result<TokenInfo> {
	let mut token_info = TokenInfo::new();
	for address in addresses {
		let symbol =
			match azero_contracts::psp22::read::read_symbol(rpc_client, &address, None).await? {
				Ok(Some(symbol)) => Some(symbol),
				_ => None,
			};
        let name = 
            match azero_contracts::psp22::read::read_name(rpc_client, &address, None).await? {
                Ok(Some(name)) => Some(name),
                _ => None,
            };
		let decimals =
			match azero_contracts::psp22::read::read_decimals(rpc_client, &address, None).await? {
				Ok(decimals) => decimals,
				_ => 0,
			};
		token_info.add_token(address.clone(), symbol, name, decimals);
	}
	Ok(token_info)
}

pub fn wazero() -> AccountId {
	AccountId::from_str("5CtuFVgEUz13SFPVY6s2cZrnLDEkxQXc19aXrNARwEBeCXgg").unwrap()
}

pub fn usdt() -> AccountId {
	AccountId::from_str("5Et3dDcXUiThrBCot7g65k3oDSicGy4qC82cq9f911izKNtE").unwrap()
}

pub fn usdc() -> AccountId {
	AccountId::from_str("5FYFojNCJVFR2bBNKfAePZCa72ZcVX5yeTv8K9bzeUo8D83Z").unwrap()
}

pub fn weth() -> AccountId {
	AccountId::from_str("5EoFQd36196Duo6fPTz2MWHXRzwTJcyETHyCyaB3rb61Xo2u").unwrap()
}

pub fn wbtc() -> AccountId {
	AccountId::from_str("5EEtCdKLyyhQnNQWWWPM1fMDx1WdVuiaoR9cA6CWttgyxtuJ").unwrap()
}

fn address_to_canonical_symbol(address: &AccountId) -> Option<String> {
	if address == &wazero() {
		Some("AZERO".to_string())
	} else if address == &usdt() {
		Some("USDT".to_string())
	} else if address == &usdc() {
		Some("USDC".to_string())
	} else if address == &weth() {
		Some("ETH".to_string())
	} else if address == &wbtc() {
		Some("BTC".to_string())
	} else {
		None
	}
}

pub fn get_price_by_token_address(token: &AccountId, price_feed: &PriceFeed) -> Option<f64> {
    address_to_canonical_symbol(token)
        .map(|s| price_feed.get_price(&s))
        .flatten()
}
