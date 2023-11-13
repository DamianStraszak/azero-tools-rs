use std::sync::Arc;

use anyhow::Result;

use crate::{
    contracts::{
        events::{backwards_compatible_into_contract_event, GenericContractEvent},
        info::backwards_compatible_get_contract_infos,
    },
    initialize_client,
    psp22::{read_decimals, read_name, read_symbol, read_total_supply},
    storage_calls::{
        get_contract_state_root_from_trie_id, get_contract_storage_from_trie_id,
        storage_to_balances,
    },
    token_db::ContractKind,
    Client,
};
use futures::StreamExt;
use parking_lot::Mutex;
use priority_queue::PriorityQueue;
use std::hash::{Hash, Hasher};
use subxt::utils::AccountId32;

use super::{ContractInfo, PSP22Contract, PSP22ContractMetadata, TokenDB};

pub struct TokenDBTracker {
    db: TokenDB,
    endpoint: String,
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
    let root_hash = get_contract_state_root_from_trie_id(api, info.trie_id.clone(), None).await?;
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
    let storage = get_contract_storage_from_trie_id(api, trie_id, true, None).await?;
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
    Ok(contracts.into_keys().collect())
}

const FREQUENCY_SAVE_BACKUP_SECS: u64 = 600;
const BREAK_TIME_MILLIS: u64 = 30;

#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) struct AccountId32HashWrapper(pub(crate) AccountId32);

impl Hash for AccountId32HashWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0 .0.hash(state);
    }
}

#[derive(Clone)]
struct AccountPQ {
    queue: Arc<Mutex<PriorityQueue<AccountId32HashWrapper, u32>>>,
}

impl AccountPQ {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(PriorityQueue::new())),
        }
    }

    pub fn insert_or_update(&self, address: AccountId32, priority: u32) {
        let mut queue = self.queue.lock();
        queue.push_increase(AccountId32HashWrapper(address.clone()), priority);
    }

    pub fn pop(&self) -> Option<(AccountId32, u32)> {
        let mut queue = self.queue.lock();
        queue.pop().map(|(k, p)| (k.0, p))
    }

    pub fn len(&self) -> usize {
        let queue = self.queue.lock();
        queue.len()
    }
}

async fn signal_contract_events(endpoint: &str, queue: AccountPQ) -> ! {
    loop {
        let client = initialize_client(endpoint).await;
        let mut block_stream = match client.blocks().subscribe_finalized().await {
            Ok(stream) => stream,
            Err(e) => {
                log::error!("Error subscribing to blocks: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }
        };
        loop {
            match block_stream.next().await {
                Some(block) => {
                    let block = match block {
                        Ok(b) => b,
                        Err(e) => {
                            log::error!("Error getting block from stream {}", e);
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            continue;
                        }
                    };
                    let block_number = block.header().number;
                    let block_hash = block.hash();
                    log::debug!("Stream: block {} {}", block_number, block_hash);
                    let events = match block.events().await {
                        Ok(events) => events,
                        Err(e) => {
                            log::error!("Error getting events from block: {}", e);
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            continue;
                        }
                    };
                    for event in events.iter() {
                        match event {
                            Ok(event) => {
                                if let Some(e) = backwards_compatible_into_contract_event(event) {
                                    use GenericContractEvent::*;
                                    match e {
                                        Instantiated { contract, .. } => {
                                            log::info!(
                                                "Adding contract {} to queue because Instantiated",
                                                contract
                                            );
                                            queue.insert_or_update(contract, 1);
                                        }
                                        Called { contract, .. } => {
                                            log::info!(
                                                "Adding contract {} to queue because Called",
                                                contract
                                            );
                                            queue.insert_or_update(contract, 1);
                                        }
                                        DelegateCalled { contract, .. } => {
                                            log::info!("Adding contract {} to queue because DelegateCalled", contract);
                                            queue.insert_or_update(contract, 1);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Error decoding event: {}", e);
                            }
                        }
                    }
                }
                None => {
                    log::error!("Block stream ended");
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    break;
                }
            }
        }
    }
}

impl TokenDBTracker {
    pub async fn new(db: TokenDB, backup_path: &str, endpoint: &str) -> Result<Self> {
        Ok(Self {
            db,
            endpoint: endpoint.to_string(),
            backup_path: backup_path.to_string(),
        })
    }

    pub async fn run(&self) -> ! {
        let queue = AccountPQ::new();
        let mut last_db_update = std::time::Instant::now();
        let queue_cloned = queue.clone();
        let url = self.endpoint.clone();
        tokio::spawn(async move { signal_contract_events(&url, queue_cloned).await });
        let mut client = initialize_client(&self.endpoint).await;
        let mut fail_tracker = 0;
        let mut iter_no: u64 = 0;
        loop {
            if fail_tracker >= 10 {
                fail_tracker = 0;
                log::warn!("Initializing client again after 10 failures");
                client = initialize_client(&self.endpoint).await;
            }
            if let Some((address, prio)) = queue.pop() {
                iter_no += 1;
                if iter_no % 100 == 0 {
                    log::info!("{} contracts left in queue", queue.len());
                }
                let old_info = self.db.inner.read().contracts.get(&address).cloned();
                match get_contract(&client, &address, old_info).await {
                    Ok(contract) => {
                        let mut db = self.db.inner.write();
                        db.contracts.insert(address, contract);
                    }
                    Err(e) => {
                        log::debug!("Error updating contract {}: {}", address, e);
                        fail_tracker += 1;
                    }
                }
                if prio == 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(BREAK_TIME_MILLIS)).await;
                }
            } else {
                log::info!("Starting a new cycle over all contracts");
                match get_current_contracts(&client).await {
                    Ok(contracts) => {
                        for c in contracts {
                            queue.insert_or_update(c, 0);
                        }
                    }
                    Err(e) => {
                        log::error!("Error {} getting contracts", e);
                        fail_tracker += 1;
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
