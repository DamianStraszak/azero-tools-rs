use askama::Template;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{collections::BTreeMap, fs, ops::Deref, sync::Arc};
use subxt::utils::{AccountId32, H256};
pub type CodeHash = [u8; 32];

mod serialization;
pub mod tracker;

use serialization::{de_u128_from_string, deserialize_map, ser_u128_as_string, serialize_map};

#[derive(Clone)]
pub struct TokenDB {
    inner: Arc<RwLock<TokenDBInner>>,
}

impl TokenDB {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(TokenDBInner::new())),
        }
    }

    pub fn from_disk(filepath: &str) -> Self {
        let inner = match TokenDBInner::read_from_disk(filepath) {
            Ok(inner) => inner,
            Err(e) => {
                log::warn!("Failed to read token DB from disk: {}", e);
                TokenDBInner::new()
            }
        };
        TokenDB {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub fn get_summary(&self, network: String) -> DbSummary {
        self.inner.read().get_summary(network)
    }

    pub fn get_account_details(
        &self,
        network: String,
        account_id: String,
    ) -> AccountDetailsWrapper {
        let maybe_account_details = match AccountId32::from_str(&account_id) {
            Ok(account) => MaybeAccountDetails::Ok(self.inner.read().get_account_details(&account)),
            Err(_) => MaybeAccountDetails::Incorrect(account_id),
        };
        AccountDetailsWrapper {
            maybe_account: maybe_account_details,
            network,
        }
    }

    pub fn clone_inner(&self) -> TokenDBInner {
        self.inner.read().clone()
    }
}

impl Deref for TokenDB {
    type Target = Arc<RwLock<TokenDBInner>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TokenDBInner {
    pub contracts: BTreeMap<AccountId32, ContractInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PSP22ContractMetadata {
    name: Option<String>,
    symbol: Option<String>,
    decimals: u8,
}

impl Default for PSP22ContractMetadata {
    fn default() -> Self {
        Self {
            name: None,
            symbol: None,
            decimals: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PSP22Contract {
    #[serde(
        serialize_with = "ser_u128_as_string",
        deserialize_with = "de_u128_from_string"
    )]
    total_supply: u128,
    metadata: Option<PSP22ContractMetadata>,
    #[serde(serialize_with = "serialize_map", deserialize_with = "deserialize_map")]
    holders: BTreeMap<AccountId32, u128>,
}

const MAX_SYMBOL_LEN: usize = 16;
const MAX_NAME_LEN: usize = 32;

impl PSP22Contract {
    pub fn symbol_to_display(&self) -> String {
        let mut symbol = self
            .metadata
            .as_ref()
            .map(|m| m.symbol.clone())
            .unwrap_or_default()
            .unwrap_or_else(|| "UNKNOWN".to_string());
        symbol.truncate(MAX_SYMBOL_LEN);
        symbol
    }

    pub fn name_to_display(&self) -> String {
        let mut name = self
            .metadata
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_default()
            .unwrap_or_else(|| "UNKNOWN".to_string());
        name.truncate(MAX_NAME_LEN);
        name
    }

    pub fn decimals(&self) -> u8 {
        self.metadata
            .as_ref()
            .map(|m| m.decimals)
            .unwrap_or_default()
    }

    pub fn human_format_amount(&self, amount: u128) -> String {
        let decimals = self.decimals();
        let amount_f64 = amount as f64 / (10f64.powi(decimals as i32));
        format!("{:.3}", amount_f64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractKind {
    PSP22(PSP22Contract),
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractInfo {
    address: AccountId32,
    root_hash: Option<Vec<u8>>,
    code_hash: H256,
    kind: ContractKind,
}

const MAX_TOKENS_IN_DB_SUMMARY: usize = 100;
const MAX_HOLDERS_IN_TOKEN_DETAILS: usize = 100;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Template)]
#[template(path = "db_summary.html")]
pub struct DbSummary {
    pub total_contracts: u32,
    pub total_psp22: u32,
    pub token_summaries: Vec<TokenSummary>,
    pub network: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenSummary {
    pub address: AccountId32,
    pub total_supply_human: String,
    pub total_holders: u32,
    pub decimals: u8,
    pub name: String,
    pub symbol: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenHolding {
    pub token_address: AccountId32,
    pub token_symbol: String,
    pub amount_human: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holder {
    pub holder_address: AccountId32,
    pub amount_human: String,
    pub amount: u128,
    pub percentage_formatted: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractDetails {
    PSP22(TokenDetails),
    Other,
    NotContract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Template)]
#[template(path = "account_details.html")]
pub struct AccountDetailsWrapper {
    maybe_account: MaybeAccountDetails,
    network: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaybeAccountDetails {
    Incorrect(String),
    Ok(AccountDetails),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountDetails {
    pub address: AccountId32,
    pub contract: ContractDetails,
    pub holdings: Vec<TokenHolding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenDetails {
    pub summary: TokenSummary,
    pub holders: Vec<Holder>,
}

impl From<(&AccountId32, &PSP22Contract)> for TokenDetails {
    fn from(apsp22: (&AccountId32, &PSP22Contract)) -> Self {
        let (_, psp22) = apsp22;
        let summary = TokenSummary::from(apsp22);
        let mut holders = Vec::new();
        for (holder_address, amount) in psp22.holders.iter() {
            let percentage = *amount as f64 / (psp22.total_supply as f64 + 1e-9) * 100.0;
            let percentage_formatted = format!("{:.3}%", percentage);
            holders.push(Holder {
                holder_address: holder_address.clone(),
                percentage_formatted,
                amount_human: psp22.human_format_amount(*amount),
                amount: *amount,
            });
        }
        holders.sort_by(|a, b| a.amount.cmp(&b.amount).reverse());
        holders.truncate(MAX_HOLDERS_IN_TOKEN_DETAILS);
        Self { summary, holders }
    }
}

impl From<(&AccountId32, &PSP22Contract)> for TokenSummary {
    fn from(apsp22: (&AccountId32, &PSP22Contract)) -> Self {
        let (address, psp22) = apsp22;
        let total_holders = psp22.holders.len() as u32;
        let metadata = psp22.metadata.clone().unwrap_or_default();
        Self {
            address: address.clone(),
            total_supply_human: psp22.human_format_amount(psp22.total_supply),
            total_holders,
            decimals: metadata.decimals,
            name: psp22.name_to_display(),
            symbol: psp22.symbol_to_display(),
        }
    }
}

impl TokenDBInner {
    pub fn new() -> Self {
        Self {
            contracts: BTreeMap::new(),
        }
    }

    pub fn read_from_disk(filepath: &str) -> anyhow::Result<TokenDBInner> {
        // Read the JSON data from the specified file path.
        let json_data = fs::read_to_string(filepath)?;

        // Deserialize the JSON string back to a TokenDBInner instance.
        let data: TokenDBInner = serde_json::from_str(&json_data)?;
        Ok(data)
    }

    pub fn write_json_to_disk(&self, filepath: &str) -> std::io::Result<()> {
        // Serialize data to a pretty-printed JSON string.
        let json_data =
            serde_json::to_string_pretty(self).expect("Failed to serialize data to JSON");

        // Write the JSON data to the specified file path.
        fs::write(filepath, json_data)
    }

    pub fn get_summary(&self, network: String) -> DbSummary {
        let total_contracts = self.contracts.len() as u32;
        let mut total_psp22 = 0;
        let mut token_summaries = Vec::new();
        for (_, info) in self.contracts.iter() {
            match &info.kind {
                ContractKind::PSP22(psp22c) => {
                    total_psp22 += 1;
                    let token_summary = TokenSummary::from((&info.address, psp22c));
                    token_summaries.push(token_summary);
                }
                _ => (),
            }
        }
        token_summaries.sort_by(|a, b| a.total_holders.cmp(&b.total_holders).reverse());
        token_summaries.truncate(MAX_TOKENS_IN_DB_SUMMARY);
        DbSummary {
            total_contracts,
            total_psp22,
            token_summaries,
            network,
        }
    }

    pub fn get_account_details(&self, account: &AccountId32) -> AccountDetails {
        let contract = match self.contracts.get(account) {
            Some(info) => match &info.kind {
                ContractKind::PSP22(psp22) => {
                    let details = TokenDetails::from((account, psp22));
                    ContractDetails::PSP22(details)
                }
                _ => ContractDetails::Other,
            },
            None => ContractDetails::NotContract,
        };
        AccountDetails {
            address: account.clone(),
            contract,
            holdings: self.get_holdings(account),
        }
    }

    fn get_holdings(&self, user: &AccountId32) -> Vec<TokenHolding> {
        let mut holdings = Vec::new();
        for (contract, info) in self.contracts.iter() {
            match &info.kind {
                ContractKind::PSP22(psp22) => {
                    if let Some(balance) = psp22.holders.get(user) {
                        if balance > &0 {
                            let token_symbol = psp22
                                .metadata
                                .as_ref()
                                .cloned()
                                .unwrap_or_default()
                                .symbol
                                .unwrap_or_else(|| "UNKNOWN".to_string());
                            holdings.push(TokenHolding {
                                token_address: contract.clone(),
                                token_symbol,
                                amount_human: psp22.human_format_amount(*balance),
                            });
                        }
                    }
                }
                _ => (),
            }
        }
        holdings
    }
}
