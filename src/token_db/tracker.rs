use anyhow::Result;

use rand::{seq::SliceRandom, thread_rng};
use subxt::utils::AccountId32;

use crate::{
    contracts::info::backwards_compatible_get_contract_infos,
    psp22::{read_decimals, read_name, read_symbol, read_total_supply},
    storage_calls::{
        get_contract_state_root_from_trie_id, get_contract_storage_from_trie_id,
        storage_to_balances,
    },
    token_db::ContractKind,
    Client,
};

use super::{ContractInfo, PSP22Contract, PSP22ContractMetadata, TokenDB};

pub struct TokenDBTracker {
    db: TokenDB,
    api: Client,
    backup_path: String,
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
    let info =
        match crate::contracts::info::backwards_compatible_get_contract_info(api, address).await? {
            Some(info) => info,
            None => return Err(anyhow::anyhow!("No contract info for {}", address)),
        };
    let root_hash = get_contract_state_root_from_trie_id(&api, info.trie_id.clone(), None).await?;
    log::debug!("Getting total_supply for contract {}", address);
    let total_supply = match read_total_supply(api, address).await? {
        Ok(total_supply) => total_supply,
        Err(e) => {
            log::debug!("No total suppply for {} {:?}", address, e);
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
    let trie_id = info.trie_id;
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
    let contracts = backwards_compatible_get_contract_infos(api).await?;
    Ok(contracts.into_iter().map(|(k, _)| k).collect())
}

const FREQUENCY_SAVE_BACKUP_SECS: u64 = 60;

impl TokenDBTracker {
    pub async fn new(db: TokenDB, backup_path: &str, endpoint: &str) -> Result<Self> {
        let api = Client::from_url(endpoint).await?;
        Ok(Self {
            db,
            api,
            backup_path: backup_path.to_string(),
        })
    }

    pub async fn run(&self) -> ! {
        let mut contracts_to_update: Vec<AccountId32> = Vec::new();
        let mut last_db_update = std::time::Instant::now();
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            if let Some(address) = contracts_to_update.pop() {
                if contracts_to_update.len() % 100 == 0 {
                    log::info!("{} contracts left in queue", contracts_to_update.len());
                }
                let old_info = self.db.inner.read().contracts.get(&address).cloned();
                match get_contract(&self.api, &address, old_info).await {
                    Ok(contract) => {
                        let mut db = self.db.inner.write();
                        db.contracts.insert(address, contract);
                    }
                    Err(e) => {
                        log::debug!("Error updating contract {}: {}", address, e);
                    }
                }
            } else {
                log::info!("Starting a new cycle over all contracts");
                match get_current_contracts(&self.api).await {
                    Ok(contracts) => {
                        contracts_to_update = contracts;
                        let mut rng = thread_rng();
                        contracts_to_update.shuffle(&mut rng);
                    }
                    Err(e) => {
                        log::error!("Error {} getting contracts", e);
                    }
                }
            }
            let now = std::time::Instant::now();
            if now.saturating_duration_since(last_db_update).as_secs() > FREQUENCY_SAVE_BACKUP_SECS
            {
                match self.db.read().write_json_to_disk(&self.backup_path) {
                    Ok(_) => {}
                    Err(e) => log::error!("Error saving backup: {}", e),
                }
                last_db_update = now;
            }
        }
    }
}
