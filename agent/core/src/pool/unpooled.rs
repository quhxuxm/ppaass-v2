#![allow(unused)]
use crate::bo::config::Config;
use crate::crypto::AgentRsaCryptoHolder;
use crate::error::AgentError;
use crate::pool::parse_proxy_address;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::debug;
pub struct UnPooled {
    config: Arc<Config>,
    proxy_addresses: Arc<Vec<SocketAddr>>,
    rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
}
impl UnPooled {
    pub async fn new(
        config: Arc<Config>,
        rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
    ) -> Result<Self, AgentError> {
        let proxy_addresses = Arc::new(parse_proxy_address(&config)?);
        Ok(Self {
            config,
            proxy_addresses,
            rsa_crypto_holder,
        })
    }
    pub async fn take_proxy_connection(&self) -> Result<TcpStream, AgentError> {
        debug!("Create un-pooled proxy connection");
        Ok(TcpStream::connect(self.proxy_addresses.as_slice()).await?)
    }
    pub async fn return_proxy_connection(
        &self,
        _proxy_tcp_stream: TcpStream,
    ) -> Result<(), AgentError> {
        Ok(())
    }
}


