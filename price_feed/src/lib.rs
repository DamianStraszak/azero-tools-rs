use std::{collections::BTreeMap, sync::Arc};

use anyhow::Result;
use parking_lot::Mutex;
use paste::paste;

#[derive(Clone)]
pub struct PriceFeed {
	prices: Arc<Mutex<BTreeMap<String, f64>>>,
}

const ORACLE_URLS: &str = include_str!("oracle_urls.json");

#[derive(Debug, Clone, serde::Deserialize)]
struct OracleUrlsJson {
	dia: Vec<OracleUrlJson>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct OracleUrlJson {
	symbol: String,
	url: String,
}

fn get_oracle_urls() -> BTreeMap<String, String> {
	let json: OracleUrlsJson = serde_json::from_str(ORACLE_URLS).unwrap();
	json.dia.into_iter().map(|o| (o.symbol, o.url)).collect()
}

async fn get_price_from_url(url: &str) -> Result<f64> {
	let resp = reqwest::get(url).await?;
	let json = resp.json::<serde_json::Value>().await?;
	let price = json["Price"].as_f64().unwrap();
	Ok(price)
}

macro_rules! generate_getters {
    ($($token:ident),+) => {
        paste! {
            $(
                #[allow(non_snake_case)]
                pub fn [<get_ $token>](&self) -> f64 {
                    self.get_price(stringify!($token)).unwrap()
                }
            )+
        }
    };
}

impl PriceFeed {
	pub async fn new() -> Result<Self> {
		let urls = get_oracle_urls();
		let mut prices = BTreeMap::new();
		for (symbol, url) in urls.iter() {
			let price = get_price_from_url(url).await?;
			prices.insert(symbol.clone(), price);
		}
		let prices = Arc::new(Mutex::new(prices));
		tokio::spawn(keep_updating(prices.clone(), urls));
		Ok(Self { prices })
	}

	pub fn get_price(&self, symbol: &str) -> Option<f64> {
		self.prices.lock().get(symbol).cloned()
	}

	generate_getters!(AZERO, USDC, USDT, ETH, BTC);
}

async fn keep_updating(
	prices: Arc<Mutex<BTreeMap<String, f64>>>,
	urls: BTreeMap<String, String>,
) -> ! {
	loop {
		for (symbol, url) in urls.iter() {
			let p = get_price_from_url(url).await;
			match p {
				Ok(p) => *prices.lock().get_mut(symbol).unwrap() = p,
				Err(e) => {
					log::info!("Error updating price for {}: {:?}", symbol, e);
					tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
				},
			}
			tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
		}
		tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
	}
}
