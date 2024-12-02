#![allow(unused)]
use crate::bo::config::Config;
use crate::crypto::AgentRsaCryptoHolder;
use crate::error::AgentError;
use crate::pool::{parse_proxy_address, PooledProxyConnection};
use rand::random;
use socket2::{Domain, Protocol, Socket, TcpKeepalive, Type};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
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
    pub async fn take_proxy_connection(
        &self,
    ) -> Result<PooledProxyConnection<TcpStream>, AgentError> {
        debug!("Create un-pooled proxy connection");
        let proxy_socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
        let random_proxy_addr_index = random::<usize>() % self.proxy_addresses.len();
        proxy_socket.connect_timeout(
            &self.proxy_addresses[random_proxy_addr_index].into(),
            Duration::from_secs(*self.config.proxy_connect_timeout()),
        )?;
        proxy_socket.set_nonblocking(true)?;
        proxy_socket.set_reuse_address(true)?;
        proxy_socket.set_keepalive(true)?;
        let keepalive = TcpKeepalive::new().with_interval(Duration::from_secs(*self.config.proxy_connection_tcp_keepalive_interval()))
            .with_time(Duration::from_secs(*self.config.proxy_connection_tcp_keepalive_time()));
        #[cfg(target_os = "linux")]
        keepalive.with_retries(*self.config.proxy_connection_tcp_keepalive_retry());
        proxy_socket.set_tcp_keepalive(&keepalive)?;
        proxy_socket.set_nonblocking(true)?;
        proxy_socket.set_nodelay(true)?;
        proxy_socket.set_read_timeout(Some(Duration::from_secs(
            *self.config.proxy_connection_read_timeout(),
        )))?;
        proxy_socket.set_write_timeout(Some(Duration::from_secs(
            *self.config.proxy_connection_write_timeout(),
        )))?;
        Ok(PooledProxyConnection::new(
            TcpStream::from_std(proxy_socket.into())?,
            self.config.clone(),
        ))
    }
    pub async fn return_proxy_connection(
        &self,
        _proxy_tcp_stream: PooledProxyConnection<TcpStream>,
    ) -> Result<(), AgentError> {
        Ok(())
    }
}
