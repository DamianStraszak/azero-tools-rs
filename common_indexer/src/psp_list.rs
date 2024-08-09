use std::sync::Arc;

use parking_lot::Mutex;
use serde::Deserialize;
use std::str::FromStr;

use crate::AccountId;

const PSP_LIST_URL: &str = "https://api.psplist.xyz/tokens";

// Not all fields are present, but we don't need all of them.
#[derive(Debug, Deserialize)]
struct Token {
	#[serde(rename = "contractAddress")]
	contract_address: String,
}

const LEGACY_PSP_LIST: [&str; 11] = [
	"5CtuFVgEUz13SFPVY6s2cZrnLDEkxQXc19aXrNARwEBeCXgg",
	"5EA7h2xCP9TkAwEQ8Km2b7aQChPKVCqcS2BJWqYavoXiEsfx",
	"5EEtCdKLyyhQnNQWWWPM1fMDx1WdVuiaoR9cA6CWttgyxtuJ",
	"5ESKJbkpVa1ppUCmrkCmaZDHqm9SHihws9Uqqsoi4VrDCDLE",
	"5EoFQd36196Duo6fPTz2MWHXRzwTJcyETHyCyaB3rb61Xo2u",
	"5Et3dDcXUiThrBCot7g65k3oDSicGy4qC82cq9f911izKNtE",
	"5F9aiiwLMPC6fFxxwqHvJpm7h5T4Xm93mJT6cpDrQnKkLFoK",
	"5FYFojNCJVFR2bBNKfAePZCa72ZcVX5yeTv8K9bzeUo8D83Z",
	"5GCubYQbm9x6TQbthbWpUVrgEibXMDXhgisw8DFYCpPJQ5f7",
	"5GVjxVdUMr5dQX9TSvvwWq42jyRaXLN65MDh4A8jhdG4Rz1A",
	"5HZxA385SYeydqZUpTeKj7D37T1bL9N6JA7Xde5QMP8qiSym",
];

#[derive(Clone)]
pub struct PSPList {
	psp_list: Arc<Mutex<Vec<AccountId>>>,
}

impl PSPList {
	pub fn new() -> Self {
		let psp_list = LEGACY_PSP_LIST.iter().map(|x| AccountId::from_str(x).unwrap()).collect();
		let psp_list = Arc::new(Mutex::new(psp_list));
		tokio::spawn(keep_updating(psp_list.clone()));
		PSPList { psp_list }
	}

	pub fn get(&self) -> Vec<AccountId> {
		self.psp_list.lock().clone()
	}
}

async fn fetch_psp_list() -> anyhow::Result<Vec<AccountId>> {
	// Fetch the PSP list from the server
	let response = reqwest::get(PSP_LIST_URL).await?.json::<Vec<Token>>().await?;

	let contract_addresses: Vec<String> =
		response.into_iter().map(|token| token.contract_address).collect();
	let res = contract_addresses.iter().map(|x| AccountId::from_str(x).unwrap()).collect();
	Ok(res)
}

async fn keep_updating(psp_list: Arc<Mutex<Vec<AccountId>>>) -> ! {
	loop {
		let new_psp_list = fetch_psp_list().await;
		match new_psp_list {
			Ok(new_psp_list) => {
				*psp_list.lock() = new_psp_list;
			},
			Err(e) => {
				log::error!("Failed to fetch PSP list: {:?}", e);
			},
		}
		tokio::time::sleep(std::time::Duration::from_secs(60)).await;
	}
}
