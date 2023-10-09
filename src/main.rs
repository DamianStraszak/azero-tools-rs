use azero::runtime_types::pallet_contracts::storage::ContractInfo;
use subxt::rpc::RpcClient;
use subxt::rpc::types::Bytes;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig, rpc_params};

type Client = OnlineClient<PolkadotConfig>;

#[subxt::subxt(runtime_metadata_path = "./metadata/azero-mainnet.scale")]
pub mod azero {}

pub type BlockHash = <PolkadotConfig as subxt::Config>::Hash;

const WS_AZERO_MAINNET: &str = "wss://ws.azero.dev:443";

fn contract_info_of_key_to_account_id(key: &Vec<u8>) -> AccountId32 {
    let account_bytes = key[40..].to_vec();
    let array_u8: [u8; 32] = account_bytes.as_slice().try_into().unwrap();
    let account = AccountId32::from(array_u8);
    account
}

// def get_contract_storage(self, address, omit_hash, block_hash):
//         trie_id = self.get_contract_info_of(address, block_hash)['trie_id']
//         child_trie_key = CHILD_STORAGE_PREFIX+trie_id[2:]
//         BATCH_SIZE = 96
//         last_key = None
//         kvs = {}
//         while True:
//             keys = self.chain.rpc_request('childstate_getKeysPaged', [child_trie_key, "0x", BATCH_SIZE, last_key, block_hash])['result']
//             values = self.chain.rpc_request('childstate_getStorageEntries', [child_trie_key, keys,block_hash])['result']
//             for (k,v) in zip(keys, values):
//                 if omit_hash:
//                     kvs[k[34:]]=v[2:]
//                 else:
//                     kvs[k[2:]]=v[2:]
//             if len(keys) < BATCH_SIZE:
//                 break
//             last_key = keys[-1]
//         return kvs


async fn get_contract_storage(api: &Client, address: AccountId32, omit_hash: bool, block_hash: Option<BlockHash>) -> HashMap<Vec<u8>, Vec<u8>> {
    let mut res = HashMap::new();
    let contract_info = get_contract_info(&api, address).await.unwrap();
    let trie_id = contract_info.trie_id.0;
    let child_storage_prefix = "0x".to_owned() + &hex::encode(":child_storage:default:".as_bytes());
    println!("trie_id {:?}, child_storage_prefix {}", trie_id, child_storage_prefix);
    let child_trie_key = child_storage_prefix.to_owned() + &hex::encode(trie_id);
    println!("child_trie_key {}", child_trie_key);
    let batch_size = 96;
    let mut last_key: Option<String> = None;
    loop {
        let params = rpc_params![child_trie_key.clone(), "0x", batch_size, last_key, block_hash.clone()];
        let keys: Vec<Bytes> = api.rpc().request(
            "childstate_getKeysPaged",params
        )
        .await.unwrap();
        //println!("keys {:?} len {}", keys, keys.len());

        let params = rpc_params![child_trie_key.clone(), keys.clone(), block_hash.clone()];
        let values: Vec<Bytes> = api.rpc().request(
            "childstate_getStorageEntries",params
        )
        .await.unwrap();
        last_key = keys.last().cloned().map(|k| "0x".to_owned() + &hex::encode(k.0));
        let len = keys.len();
        for (k,v) in keys.into_iter().zip(values.into_iter()) {
            let key = if omit_hash {
                k.0[16..].to_vec()
            } else {
                k.0
            };
            res.insert(key, v.0);
        }
        if len < batch_size {
            break;
        }

        
    }
    

    println!("res_len {}", res.len());    

    

    res
    
}

pub type ContractStorage = HashMap<Vec<u8>, Vec<u8>>;

// def storage_to_balances(storage, addr):
//     # Example Key: "3b8d451dbe76858e95775a827180f88615b3a3b77f1cefd3af03e107019b310edee5d67b"
//     # Example Value: "0080c979452f13000000000000000000"
//     magic_prefixes = ["3b8d451d", "e4aae541", "264866c2"]
//     storage_72_32 = {k:v for (k,v) in storage.items() if (len(k) == 72 and len(v) == 32)}
//     prefixes = list(set([k[:8] for k in storage_72_32.keys()]))
//     if prefixes == []:
//         # print(f"No 72_32 entries for {addr}")
//         return {}
//     for magic_prefix in magic_prefixes:
//         if magic_prefix in prefixes:
//             balances = {}
//             for (k, v) in storage_72_32.items():
//                 if k.startswith(magic_prefix):
//                     account_hex = k[8:]
//                     account = ss58_encode(f"0x{account_hex}", ss58_format=42)
//                     balance_hex = v
//                     balance = balance_from_hex(balance_hex)
//                     if balance > 0:
//                         balances[account] = balance
//             return balances
//     print(f"Unexpected prefixes {prefixes} for {addr}")
//     return {}

fn storage_to_balances(storage: &ContractStorage) -> BTreeMap<AccountId32, u128> {
    let magic_prefixes: Vec<Vec<u8>> = ["3b8d451d", "e4aae541", "264866c2"]
        .iter()
        .map(|s| hex::decode(s).unwrap())
        .collect();

    let storage_36_16: HashMap<Vec<u8>, Vec<u8>> = storage
        .iter()
        .filter(|(k, v)| k.len() == 36 && v.len() == 16)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let prefixes: Vec<Vec<u8>> = storage_36_16
        .keys()
        .map(|k| k[..4].to_vec())
        .collect();
    if prefixes.is_empty() {
        return BTreeMap::new();
    }
    for magic_prefix in magic_prefixes {
        if prefixes.contains(&magic_prefix.to_owned()) {
            let mut balances = BTreeMap::new();
            for (k, v) in storage_36_16.iter() {
                if k.starts_with(&magic_prefix) {
                    //let account_hex = &k[4..];
                    let array_u8: [u8; 32] = k[4..].try_into().unwrap();
                    let account = AccountId32::from(array_u8);
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


async fn get_contract_info(api: &Client, address: AccountId32) -> Option<ContractInfo> {
    let storage_address = azero::storage().contracts().contract_info_of(address);
    api
        .storage()
        .at_latest()
        .await
        .unwrap()
        .fetch(&storage_address).await.unwrap()
}

async fn get_contract_infos(api: &Client) -> BTreeMap<AccountId32, ContractInfo> {
    let storege_address = azero::storage().contracts().contract_info_of_root();
    let mut res = BTreeMap::new();
    let mut stream = api
        .storage()
        .at_latest()
        .await
        .unwrap()
        .iter(storege_address, 200)
        .await
        .unwrap();
    while let Ok(Some((key, value))) = stream.next().await {
        let key = key.0;
        let account = contract_info_of_key_to_account_id(&key);
        res.insert(account, value);
    }
    res
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = OnlineClient::<PolkadotConfig>::from_url(WS_AZERO_MAINNET).await?;
    let block_hash = api.rpc().block_hash(Some(55555555u32.into())).await.unwrap().unwrap();
    let contract_acc = AccountId32::from_str("5GvYFrcAXqRM46djC2pgtp9vj1jtGJ99yv4r3RLejmBqAudL").unwrap();
    get_contract_storage(&api, contract_acc, true, None).await;
    let contract_acc = AccountId32::from_str("5D4doeP2gc4xeRKB4NHdGW6FNy51UbJboVMEo7QTUoqDPkJd").unwrap();
    let s=get_contract_storage(&api, contract_acc, true, Some(block_hash)).await;
    let mut balances = storage_to_balances(&s).into_iter().collect::<Vec<_>>();
    balances.sort_by(|a, b| b.1.cmp(&a.1));
    for (k, v) in balances.iter() {
        println!("{} {}", k, v);
    }
    println!("balances len {}", balances.len());

    Ok(())
}
