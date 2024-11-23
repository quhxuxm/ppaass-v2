use crate::bo::config::Config;
use crate::crypto::AgentRsaCryptoHolder;
use crate::pool::ProxyConnectionPool;
use accessory::Accessors;
use derive_builder::Builder;
use std::sync::Arc;
#[derive(Clone, Accessors, Builder)]
pub struct ServerState {
    #[access(get)]
    config: Arc<Config>,
    #[access(get)]
    rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
    #[access(get)]
    proxy_connection_pool: Arc<ProxyConnectionPool>,
}
