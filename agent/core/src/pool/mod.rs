use crate::bo::config::Config;
use crate::crypto::AgentRsaCryptoHolder;
use crate::error::AgentError;
pub use crate::pool::connection::PooledProxyConnection;
use crate::pool::pooled::Pooled;
use crate::pool::unpooled::UnPooled;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpStream;
mod connection;
mod pooled;
mod unpooled;
fn parse_proxy_address(config: &Config) -> Result<Vec<SocketAddr>, AgentError> {
    let proxy_addresses = config
        .proxy_addresses()
        .iter()
        .filter_map(|addr| SocketAddr::from_str(addr).ok())
        .collect::<Vec<SocketAddr>>();
    Ok(proxy_addresses)
}
pub enum ProxyConnectionPool {
    UnPooled(UnPooled),
    Pooled(Pooled),
}
impl ProxyConnectionPool {
    pub async fn new(
        config: Arc<Config>,
        rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
    ) -> Result<Self, AgentError> {
        match *config.proxy_connection_pool_size() {
            None => Ok(Self::UnPooled(
                UnPooled::new(config, rsa_crypto_holder).await?,
            )),
            Some(pool_size) => Ok(Self::Pooled(
                Pooled::new(config, pool_size, rsa_crypto_holder).await?,
            )),
        }
    }
    pub async fn take_proxy_connection(
        &self,
    ) -> Result<PooledProxyConnection<TcpStream>, AgentError> {
        match self {
            ProxyConnectionPool::UnPooled(un_pooled) => un_pooled.take_proxy_connection().await,
            ProxyConnectionPool::Pooled(pooled) => pooled.take_proxy_connection().await,
        }
    }
    pub async fn return_proxy_connection(
        &self,
        proxy_tcp_stream: PooledProxyConnection<TcpStream>,
    ) -> Result<(), AgentError> {
        match self {
            ProxyConnectionPool::UnPooled(un_pooled) => {
                un_pooled.return_proxy_connection(proxy_tcp_stream).await
            }
            ProxyConnectionPool::Pooled(pooled) => {
                pooled.return_proxy_connection(proxy_tcp_stream).await
            }
        }
    }
}
