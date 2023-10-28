use anyhow::Result;

use primitive_types::H256;
use rand::{seq::SliceRandom, thread_rng};
use subxt::utils::AccountId32;

use crate::{
    azero::contracts,
    psp22::{read_decimals, read_name, read_symbol, read_total_supply},
    storage_calls::{
        get_contract_info, get_contract_infos, get_contract_state_root_from_trie_id,
        get_contract_storage_from_trie_id, storage_to_balances,
    },
    token_db::ContractKind,
    Client, WS_AZERO_MAINNET,
};

use super::{ContractInfo, PSP22Contract, PSP22ContractMetadata, TokenDB, TOKEN_DB_FILEPATH_JSON};

pub struct TokenDBTracker {
    db: TokenDB,
    api: Client,
}

async fn get_psp22_metadata(
    api: &Client,
    address: &AccountId32,
) -> Result<Option<PSP22ContractMetadata>> {
    let decimals = if let Ok(decimals) = read_decimals(api, address).await? {
        decimals
    } else {
        return Ok(None);
    };
    let name = if let Ok(name) = read_name(api, address).await? {
        name
    } else {
        return Ok(None);
    };
    let symbol = if let Ok(symbol) = read_symbol(api, address).await? {
        symbol
    } else {
        return Ok(None);
    };
    Ok(Some(PSP22ContractMetadata {
        decimals,
        name,
        symbol,
    }))
}

async fn get_contract(
    api: &Client,
    address: &AccountId32,
    old: Option<ContractInfo>,
) -> Result<ContractInfo> {
    let info = match get_contract_info(api, address).await? {
        Some(info) => info,
        None => return Err(anyhow::anyhow!("No contract info for {}", address)),
    };
    let root_hash =
        get_contract_state_root_from_trie_id(&api, info.trie_id.0.clone(), None).await?;
    log::debug!("Getting total_supply for contract {}", address);
    let total_supply = match read_total_supply(api, address).await? {
        Ok(total_supply) => total_supply,
        Err(e) => {
            log::info!("No total suppply for {} {:?}", address, e);
            return Ok(ContractInfo {
                address: address.clone(),
                root_hash,
                code_hash: info.code_hash,
                kind: ContractKind::Other,
            });
        }
    };
    if let Some(old) = old {
        if old.root_hash == root_hash {
            if let ContractKind::PSP22(old_psp22) = old.kind {
                log::debug!("Root match {}", address);
                return Ok(ContractInfo {
                    address: address.clone(),
                    root_hash,
                    code_hash: info.code_hash,
                    kind: ContractKind::PSP22(PSP22Contract {
                        total_supply,
                        metadata: old_psp22.metadata,
                        holders: old_psp22.holders,
                    }),
                });
            }
        }
    };

    log::debug!("NO Root match {}", address);
    log::debug!("Getting metadata for contract {}", address);
    let metadata = get_psp22_metadata(api, address).await?;
    let trie_id = info.trie_id.0;
    log::debug!("Getting storage for contract {}", address);
    let storage = get_contract_storage_from_trie_id(&api, trie_id, true, None).await?;
    log::debug!("Computing holders for contract {}", address);
    let holders = storage_to_balances(&storage);

    let kind = ContractKind::PSP22(PSP22Contract {
        total_supply,
        metadata,
        holders,
    });
    Ok(ContractInfo {
        address: address.clone(),
        root_hash,
        code_hash: info.code_hash,
        kind,
    })
}

async fn get_current_contracts(api: &Client) -> Result<Vec<AccountId32>> {
    let contracts = get_contract_infos(api).await?;
    Ok(contracts.into_iter().map(|(k, _)| k).collect())
}

impl TokenDBTracker {
    pub async fn new(db: TokenDB) -> Result<Self> {
        let api = Client::from_url(WS_AZERO_MAINNET).await?;
        Ok(Self { db, api })
    }

    pub async fn run(&self) -> Result<()> {
        let mut contracts_to_update: Vec<AccountId32> = Vec::new();
        let mut last_db_update = std::time::Instant::now();
        loop {
            if let Some(address) = contracts_to_update.pop() {
                log::info!("{} contracts left in queue", contracts_to_update.len());
                let old_info = self.db.inner.read().contracts.get(&address).cloned();
                match get_contract(&self.api, &address, old_info).await {
                    Ok(contract) => {
                        let mut db = self.db.inner.write();
                        db.contracts.insert(address, contract);
                    }
                    Err(e) => {
                        log::info!("Error updating contract {}: {}", address, e);
                    }
                }
            } else {
                log::info!("Starting a new cycle over all contracts");
                contracts_to_update = get_current_contracts(&self.api).await?;
                let mut rng = thread_rng();
                contracts_to_update.shuffle(&mut rng);
            }
            let now = std::time::Instant::now();
            if now.saturating_duration_since(last_db_update).as_secs() > 60 {
                self.db.read().write_json_to_disk(TOKEN_DB_FILEPATH_JSON)?;
                last_db_update = now;
            }
        }
    }
}
