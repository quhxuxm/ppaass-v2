use crate::bo::event::ProxyServerEvent;
use crate::config::Config;
use crate::crypto::ProxyRsaCryptoHolder;
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
    #[builder(setter(strip_option), default)]
    forward_rsa_crypto_holder: Option<Arc<ProxyRsaCryptoHolder>>,
    #[access(get)]
    server_event_tx: Arc<Sender<ProxyServerEvent>>,
}
