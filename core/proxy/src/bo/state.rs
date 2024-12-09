use crate::bo::config::Config;
use crate::bo::event::ProxyServerEvent;
use crate::crypto::{ProxyForwardRsaCryptoHolder, ProxyRsaCryptoHolder};
use accessory::Accessors;
use derive_builder::Builder;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
#[derive(Clone, Accessors, Builder)]
pub struct ServerState {
    #[access(get)]
    config: Arc<Config>,
    #[access(get)]
    rsa_crypto_holder: Arc<ProxyRsaCryptoHolder>,
    #[access(get)]
    forward_rsa_crypto_holder: Arc<ProxyForwardRsaCryptoHolder>,
    #[access(get)]
    server_event_tx: Arc<Sender<ProxyServerEvent>>,
}
