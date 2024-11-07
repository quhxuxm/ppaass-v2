use crate::bo::config::Config;
use crate::bo::session::Session;
use crate::crypto::ProxyRsaCryptoFetcher;
use accessory::Accessors;
use derive_builder::Builder;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
#[derive(Clone, Accessors, Builder)]
pub struct ServerState {
    #[access(get)]
    config: Arc<Config>,
    #[access(get)]
    rsa_crypto_fetcher: Arc<ProxyRsaCryptoFetcher>,
    #[access(get)]
    session_repository: Arc<Mutex<HashMap<String, Session>>>,
}
