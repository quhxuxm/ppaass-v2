use crate::bo::config::Config;
use crate::error::AgentError;
use deadpool::managed::{Manager, Metrics, Object, Pool, QueueMode, RecycleResult};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpSocket, TcpStream};
use tokio::time::sleep;
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
            proxy_addresses,
            config,
        }
    }
}

impl Manager for ProxyConnectionManager {
    type Type = TcpStream;
    type Error = AgentError;
    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let proxy_socket = TcpSocket::new_v4()?;
        proxy_socket.set_keepalive(true)?;
        proxy_socket.set_reuseaddr(true)?;
        proxy_socket.set_recv_buffer_size(*self.config.proxy_socket_recv_buffer_size())?;
        proxy_socket.set_send_buffer_size(*self.config.proxy_socket_send_buffer_size())?;
        proxy_socket.set_nodelay(true)?;
        let random_index = rand::random::<usize>() % self.proxy_addresses.len();
        let proxy_address = &self.proxy_addresses[random_index];
        let proxy_tcp_stream = proxy_socket.connect(*proxy_address).await?;
        debug!("Create proxy connection on: {proxy_address}");
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
