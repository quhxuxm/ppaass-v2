use crate::bo::config::Config;
use crate::error::AgentError;
use deadpool::managed::{Manager, Metrics, Object, Pool, QueueMode, RecycleResult};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::debug;
pub struct ProxyConnectionManager {
    config: Arc<Config>,
    proxy_addresses: Vec<SocketAddr>,
}

impl ProxyConnectionManager {
    pub fn new(config: Arc<Config>) -> Self {
        let proxy_addresses: Vec<SocketAddr> = config
            .proxy_addresses()
            .iter()
            .filter_map(|addr| SocketAddr::from_str(addr).ok())
            .collect::<Vec<SocketAddr>>();
        Self {
            config,
            proxy_addresses,
        }
    }
}

impl Manager for ProxyConnectionManager {
    type Type = TcpStream;
    type Error = AgentError;
    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let proxy_tcp_stream = TcpStream::connect(self.proxy_addresses.as_slice()).await?;
        Ok(proxy_tcp_stream)
    }
    async fn recycle(&self, obj: &mut Self::Type, metrics: &Metrics) -> RecycleResult<Self::Error> {
        debug!("Recycle proxy connection: {obj:?}, metrics: {metrics:?}");
        Ok(())
    }
}

pub struct ProxyConnectionPool {
    pool: Pool<ProxyConnectionManager>,
}

impl ProxyConnectionPool {
    pub fn new(config: Arc<Config>) -> Result<Self, AgentError> {
        let pool = Pool::builder(ProxyConnectionManager::new(config.clone()))
            .max_size(*config.proxy_connection_pool_size())
            .queue_mode(QueueMode::Fifo)
            .build()?;
        Ok(Self { pool })
    }

    pub async fn take_proxy_connection(&self) -> Result<TcpStream, AgentError> {
        let proxy_connection = self
            .pool
            .get()
            .await
            .map_err(|e| AgentError::ProxyConnectionPool(format!("{e:?}")))?;
        let proxy_connection = Object::take(proxy_connection);
        Ok(proxy_connection)
    }
}
