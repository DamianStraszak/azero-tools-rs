
pub type Client = azero_config::Client;
pub type RpcClient = azero_config::RpcClient;
pub type BlockHash = azero_config::BlockHash;
pub type AccountId = azero_config::AccountId;

pub mod event_db;
pub mod scraper;
pub mod multiswaps;
pub mod pools;

pub const COMMON_START_BLOCK: u32 = 84335170;

