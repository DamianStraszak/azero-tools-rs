use std::collections::BTreeMap;

use anyhow::Result;

use subxt::utils::{AccountId32, H256};

use crate::Client;

fn contract_info_of_key_to_account_id(key: &[u8]) -> AccountId32 {
    let account_bytes = key[40..].to_vec();
    let array_u8: [u8; 32] = account_bytes.as_slice().try_into().unwrap();
    AccountId32::from(array_u8)
}

mod v_12_0 {
    use std::collections::BTreeMap;

    use super::{contract_info_of_key_to_account_id, GenericContractInfo};
    use crate::{azero_12_0 as azero, Client};
    use anyhow::Result;
    use subxt::utils::AccountId32;
    impl From<azero::runtime_types::pallet_contracts::storage::ContractInfo>
        for GenericContractInfo
    {
        fn from(info: azero::runtime_types::pallet_contracts::storage::ContractInfo) -> Self {
            Self {
                trie_id: info.trie_id.0,
                code_hash: info.code_hash,
            }
        }
    }

    pub(crate) async fn get_contract_info(
        api: &Client,
        address: &AccountId32,
    ) -> Result<Option<GenericContractInfo>> {
        let storage_address = azero::storage()
            .contracts()
            .contract_info_of(address);
        let contract_info = api
            .storage()
            .at_latest()
            .await?
            .fetch(&storage_address)
            .await
            .map_err(|e| anyhow::anyhow!("Get contract info failed {:?}", e))?;
        Ok(contract_info.map(|info| info.into()))
    }

    pub(crate) async fn get_contract_infos(
        api: &Client,
    ) -> Result<BTreeMap<AccountId32, GenericContractInfo>> {
        let storege_address = azero::storage().contracts().contract_info_of_root();
        let mut res = BTreeMap::new();
        let mut stream = api
            .storage()
            .at_latest()
            .await?
            .iter(storege_address, 200)
            .await?;
        while let Ok(Some((key, value))) = stream.next().await {
            let key = key.0;
            let account = contract_info_of_key_to_account_id(&key);
            res.insert(account, value.into());
        }
        Ok(res)
    }
}

mod v_13_0 {
    use std::collections::BTreeMap;

    use super::{contract_info_of_key_to_account_id, GenericContractInfo};
    use crate::{azero_13_0 as azero, Client};
    use anyhow::Result;
    use subxt::utils::AccountId32;
    impl From<azero::runtime_types::pallet_contracts::storage::ContractInfo>
        for GenericContractInfo
    {
        fn from(info: azero::runtime_types::pallet_contracts::storage::ContractInfo) -> Self {
            Self {
                trie_id: info.trie_id.0,
                code_hash: info.code_hash,
            }
        }
    }

    pub(crate) async fn get_contract_info(
        api: &Client,
        address: &AccountId32,
    ) -> Result<Option<GenericContractInfo>> {
        let storage_address = azero::storage()
            .contracts()
            .contract_info_of(address);
        let contract_info = api
            .storage()
            .at_latest()
            .await?
            .fetch(&storage_address)
            .await
            .map_err(|e| anyhow::anyhow!("Get contract info failed {:?}", e))?;
        Ok(contract_info.map(|info| info.into()))
    }

    pub(crate) async fn get_contract_infos(
        api: &Client,
    ) -> Result<BTreeMap<AccountId32, GenericContractInfo>> {
        let storege_address = azero::storage().contracts().contract_info_of_root();
        let mut res = BTreeMap::new();
        let mut stream = api
            .storage()
            .at_latest()
            .await?
            .iter(storege_address, 200)
            .await?;
        while let Ok(Some((key, value))) = stream.next().await {
            let key = key.0;
            let account = contract_info_of_key_to_account_id(&key);
            res.insert(account, value.into());
        }
        Ok(res)
    }
}


pub async fn backwards_compatible_get_contract_infos(
    api: &Client,
) -> Result<BTreeMap<AccountId32, GenericContractInfo>> {
    let err_13_0 = match v_13_0::get_contract_infos(api).await {
        Ok(suc) => {
            return Ok(suc);
        }
        Err(e) => e,
    };

    let err_12_0 = match v_12_0::get_contract_infos(api).await {
        Ok(suc) => {
            return Ok(suc);
        }
        Err(e) => e,
    };
    Err(anyhow::anyhow!(
        "Get contract infos failed, err_13_0: {:?} err_12_0: {:?}",
        err_13_0,
        err_12_0
    ))
}

pub async fn backwards_compatible_get_contract_info(
    api: &Client,
    address: &AccountId32,
) -> Result<Option<GenericContractInfo>> {
    let err_13_0 = match v_13_0::get_contract_info(api, address).await {
        Ok(suc) => {
            return Ok(suc);
        }
        Err(e) => e,
    };
    let err_12_0 = match v_12_0::get_contract_info(api, address).await {
        Ok(suc) => {
            return Ok(suc);
        }
        Err(e) => e,
    };
    Err(anyhow::anyhow!(
        "Get contract info failed for {}, err_13_0: {:?} err_12_0: {:?}",
        address,
        err_13_0,
        err_12_0
    ))
}

pub struct GenericContractInfo {
    pub trie_id: Vec<u8>,
    pub code_hash: H256,
}
