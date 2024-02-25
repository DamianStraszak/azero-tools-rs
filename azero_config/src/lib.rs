use std::str::FromStr;

use subxt::{
	config::substrate::SubstrateExtrinsicParams, utils::AccountId32, Config as SubxtConfig,
	OnlineClient, SubstrateConfig,
};

pub const WS_AZERO_MAINNET: &str = "wss://ws.azero.dev:443";
pub const WS_AZERO_TESTNET: &str = "wss://ws.test.azero.dev:443";

pub const ALICE: &str = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

pub fn alice_acc() -> AccountId {
	AccountId::from_str(ALICE).unwrap()
}

pub enum Config {}

pub type AccountId = AccountId32;

impl subxt::Config for Config {
	type Hash = <SubstrateConfig as SubxtConfig>::Hash;
	type AccountId = AccountId;
	type Address = <SubstrateConfig as SubxtConfig>::Address; // MultiAddress
	type Signature = <SubstrateConfig as SubxtConfig>::Signature;
	type Hasher = <SubstrateConfig as SubxtConfig>::Hasher;
	type Header = <SubstrateConfig as SubxtConfig>::Header;
	type ExtrinsicParams = SubstrateExtrinsicParams<Self>;
	//type AssetId = <SubstrateConfig as SubxtConfig>::AssetId;
}

pub type SubmittableExtrinsic = subxt::tx::SubmittableExtrinsic<Config, Client>;
pub type Client = OnlineClient<Config>;
pub type BlockHash = <Config as subxt::Config>::Hash;
pub type BlockHeader = <Config as subxt::Config>::Header;
pub type BlockNumber = u32;
pub type Block = subxt::blocks::Block<Config, Client>;
pub type Signer = subxt_signer::sr25519::Keypair;
pub type Storage = subxt::storage::Storage<Config, Client>;
