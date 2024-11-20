use crate::bo::config::Config;
use crate::destination::DestinationTransport;
use crate::error::ProxyError;
use deadpool::managed::{Manager, Metrics, RecycleResult};
use ppaass_domain::address::UnifiedAddress;
use std::collections::HashMap;
use std::sync::Arc;
pub struct DestinationTransportPoolRepository {
    destination_transports: HashMap<UnifiedAddress, Arc<DestinationTransportPool>>,
}
impl DestinationTransportPoolRepository {
    pub fn new() -> Self {
        Self {
            destination_transports: HashMap::with_capacity(1024)
        }
    }
    pub fn retrieve_destination_transport(&mut self, address: &UnifiedAddress) -> Arc<DestinationTransport> {
        let destination_transport_pool = self.destination_transports.get_mut(address);
        match destination_transport_pool {
            None => {
                // let destination_transport_pool = DestinationTransportPool::new()
            }
            Some(destination_transport) => {
                //destination_transport.clone()
            }
        }
        todo!()
    }
}
pub struct DestinationTransportPool {
    destination_socket_addr: UnifiedAddress,
    config: Arc<Config>,
    is_tcp: bool,
}
impl DestinationTransportPool {
    pub fn new(destination_socket_addr: UnifiedAddress, config: Arc<Config>, is_tcp: bool) -> Self {
        Self {
            destination_socket_addr,
            config,
            is_tcp,
        }
    }
}
impl Manager for DestinationTransportPool {
    type Type = DestinationTransport;
    type Error = ProxyError;
    async fn create(&self) -> Result<Self::Type, Self::Error> {
        if self.is_tcp {
            DestinationTransport::new_tcp(&self.destination_socket_addr, self.config.clone()).await
        } else {
            DestinationTransport::new_udp(&self.destination_socket_addr).await
        }
    }
    async fn recycle(&self, obj: &mut Self::Type, metrics: &Metrics) -> RecycleResult<Self::Error> {
        todo!()
    }
}