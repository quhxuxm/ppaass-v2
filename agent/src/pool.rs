use crate::bo::config::Config;
use crate::error::AgentError;
use deadpool::managed::{Manager, Metrics, Object, Pool, QueueMode, RecycleResult};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::sleep;
use tracing::debug;
pub struct ProxyConnectionManager {
    proxy_addresses: Vec<SocketAddr>,
}

impl ProxyConnectionManager {
    pub fn new(config: Arc<Config>) -> Self {
        let proxy_addresses: Vec<SocketAddr> = config
            .proxy_addresses()
            .iter()
            .filter_map(|addr| SocketAddr::from_str(addr).ok())
            .collect::<Vec<SocketAddr>>();
        Self { proxy_addresses }
    }
}

impl Manager for ProxyConnectionManager {
    type Type = TcpStream;
    type Error = AgentError;
    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let proxy_tcp_stream = TcpStream::connect(self.proxy_addresses.as_slice()).await?;
        debug!("Create proxy connection.");
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
        let pool_clone = pool.clone();
        let target_pool_size = *config.proxy_connection_pool_size();
        tokio::spawn(async move {
            loop {
                debug!("Resizing proxy connection pool.");
                pool_clone.resize(target_pool_size);
                sleep(Duration::from_secs(5)).await;
            }
        });
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
