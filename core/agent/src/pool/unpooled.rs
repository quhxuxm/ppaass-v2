#![allow(unused)]
use crate::config::Config;
use crate::crypto::AgentRsaCryptoHolder;
use crate::error::AgentError;
use crate::pool::{resolve_proxy_address, PooledProxyConnection};
use rand::random;
use socket2::{Domain, Protocol, SockRef, Socket, TcpKeepalive, Type};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, error};
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
        let proxy_addresses = Arc::new(resolve_proxy_address(&config)?);
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
        let random_proxy_addr_index = random::<usize>() % self.proxy_addresses.len();
        let proxy_tcp_stream = match timeout(
            Duration::from_secs(*self.config.proxy_connect_timeout()),
            TcpStream::connect(self.proxy_addresses[random_proxy_addr_index]),
        )
        .await
        {
            Ok(Ok(proxy_tcp_stream)) => proxy_tcp_stream,
            Ok(Err(e)) => {
                error!("Fail connect to proxy: {e:?}");
                return Err(e.into());
            }
            Err(e) => {
                error!(
                    "Fail connect to proxy because of timeout: {}",
                    *self.config.proxy_connect_timeout()
                );
                return Err(e.into());
            }
        };
        let proxy_socket = SockRef::from(&proxy_tcp_stream);
        proxy_socket.set_reuse_address(true)?;

        if *self.config.proxy_connection_tcp_keepalive() {
            let keepalive = TcpKeepalive::new()
                .with_interval(Duration::from_secs(
                    self.config
                        .proxy_connection_tcp_keepalive_interval()
                        .ok_or(AgentError::Unknown("Fail to create proxy connection tcp socket becauause of no keepalive interval provided".to_string()))?,
                ))
                .with_time(Duration::from_secs(
                    self.config.proxy_connection_tcp_keepalive_time().ok_or(AgentError::Unknown("Fail to create proxy connection tcp socket becauause of no keepalive time provided".to_string()))?
                ));
            proxy_socket.set_tcp_keepalive(&keepalive)?;
        }

        proxy_socket.set_nodelay(true)?;
        if let Some(buffer_size) = self.config.proxy_socket_receive_buffer_size() {
            proxy_socket.set_recv_buffer_size(*buffer_size)?;
        }
        if let Some(buffer_size) = self.config.proxy_socket_send_buffer_size() {
            proxy_socket.set_send_buffer_size(*buffer_size)?;
        }
        if let Some(timeout) = self.config.proxy_connection_read_timeout() {
            proxy_socket.set_read_timeout(Some(Duration::from_secs(*timeout)))?;
        }
        if let Some(timeout) = self.config.proxy_connection_write_timeout() {
            proxy_socket.set_write_timeout(Some(Duration::from_secs(*timeout)))?;
        }
        Ok(PooledProxyConnection::new(
            proxy_tcp_stream,
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
