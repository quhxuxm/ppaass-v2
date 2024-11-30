use crate::bo::config::Config;
use crate::codec::ControlPacketCodec;
use crate::crypto::AgentRsaCryptoHolder;
use crate::error::AgentError;
use crate::pool::{parse_proxy_address, PooledProxyConnection};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use ppaass_domain::heartbeat::HeartbeatPing;
use ppaass_domain::{AgentControlPacket, ProxyControlPacket};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_util::codec::Framed;
use tracing::{debug, error};
pub struct Pooled {
    pool: Arc<Mutex<Vec<PooledProxyConnection<TcpStream>>>>,
    config: Arc<Config>,
    proxy_addresses: Arc<Vec<SocketAddr>>,
    filling_connection: Arc<AtomicBool>,
    initial_pool_size: usize,
    rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
}
impl Pooled {
    pub async fn new(
        config: Arc<Config>,
        initial_pool_size: usize,
        rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
    ) -> Result<Self, AgentError> {
        let proxy_addresses = Arc::new(parse_proxy_address(&config)?);
        let pool = Arc::new(Mutex::new(Vec::with_capacity(initial_pool_size)));
        let filling_connection = Arc::new(AtomicBool::new(false));
        {
            let pool = pool.clone();
            let proxy_addresses = proxy_addresses.clone();
            let config = config.clone();
            let filling_connection = filling_connection.clone();
            tokio::spawn(async move {
                loop {
                    Self::fill_pool(
                        pool.clone(),
                        proxy_addresses.clone(),
                        config.clone(),
                        filling_connection.clone(),
                        initial_pool_size,
                    )
                    .await;
                    sleep(Duration::from_secs(
                        *config.proxy_connection_pool_fill_interval(),
                    ))
                    .await;
                }
            });
        }
        Ok(Self {
            pool,
            config,
            proxy_addresses,
            filling_connection,
            initial_pool_size,
            rsa_crypto_holder,
        })
    }
    pub async fn take_proxy_connection(
        &self,
    ) -> Result<PooledProxyConnection<TcpStream>, AgentError> {
        Self::concrete_take_proxy_connection(
            self.pool.clone(),
            self.proxy_addresses.clone(),
            self.config.clone(),
            self.filling_connection.clone(),
            self.initial_pool_size,
            self.rsa_crypto_holder.clone(),
        )
        .await
    }
    pub async fn return_proxy_connection(
        &self,
        proxy_tcp_stream: PooledProxyConnection<TcpStream>,
    ) -> Result<(), AgentError> {
        let mut pool = self.pool.lock().await;
        pool.push(proxy_tcp_stream);
        Ok(())
    }
    async fn create_proxy_tcp_stream(
        config: Arc<Config>,
        proxy_addresses: Arc<Vec<SocketAddr>>,
        proxy_connection_tx: Sender<PooledProxyConnection<TcpStream>>,
    ) -> Result<(), AgentError> {
        let proxy_tcp_stream = TcpStream::connect(proxy_addresses.as_slice()).await?;
        debug!("Create proxy connection: {proxy_tcp_stream:?}");
        proxy_connection_tx
            .send(PooledProxyConnection::new(proxy_tcp_stream, config))
            .await
            .map_err(|_| {
                AgentError::ProxyConnectionPool("Fail to send proxy connection".to_string())
            })?;
        Ok(())
    }
    async fn concrete_take_proxy_connection(
        pool: Arc<Mutex<Vec<PooledProxyConnection<TcpStream>>>>,
        proxy_addresses: Arc<Vec<SocketAddr>>,
        config: Arc<Config>,
        filling_connection: Arc<AtomicBool>,
        pool_size: usize,
        rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
    ) -> Result<PooledProxyConnection<TcpStream>, AgentError> {
        loop {
            let pool_clone = pool.clone();
            let mut pool = pool.lock().await;
            let current_pool_size = pool.len();
            debug!("Taking proxy connection, current pool size: {current_pool_size}");
            let proxy_connection = pool.pop();
            drop(pool);
            match proxy_connection {
                None => {
                    debug!("No proxy connection available, current pool size: {current_pool_size}");
                    Self::fill_pool(
                        pool_clone,
                        proxy_addresses.clone(),
                        config.clone(),
                        filling_connection.clone(),
                        pool_size,
                    )
                    .await;
                    sleep(Duration::from_millis(100)).await;
                    continue;
                }
                Some(mut proxy_connection) => {
                    debug!("Proxy connection available, current pool size before take: {current_pool_size}");
                    if !proxy_connection.need_check() {
                        return Ok(proxy_connection);
                    } else {
                        match Self::check_proxy_connection(
                            &mut proxy_connection,
                            config.clone(),
                            rsa_crypto_holder.clone(),
                        )
                        .await
                        {
                            Ok(()) => return Ok(proxy_connection),
                            Err(e) => {
                                error!("Failed to check proxy connection: {e}");
                                continue;
                            }
                        }
                    }
                }
            }
        }
    }
    async fn check_proxy_connection(
        proxy_connection: &mut PooledProxyConnection<TcpStream>,
        config: Arc<Config>,
        rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
    ) -> Result<(), AgentError> {
        debug!("Checking proxy connection : {proxy_connection:?}");
        let config = config.clone();
        let rsa_crypto_holder = rsa_crypto_holder.clone();
        let mut proxy_ctl_framed = Framed::new(
            proxy_connection,
            ControlPacketCodec::new(config.auth_token().to_owned(), rsa_crypto_holder.clone()),
        );
        proxy_ctl_framed
            .send(AgentControlPacket::Heartbeat(HeartbeatPing {
                heartbeat_time: Utc::now(),
            }))
            .await?;
        let pong_packet = match proxy_ctl_framed.next().await {
            None => {
                error!("Proxy connection closed already.");
                return Err(AgentError::ProxyConnectionExhausted);
            }
            Some(Err(e)) => {
                error!("Fail to receive heartbeat pong from proxy: {e:?}");
                return Err(e);
            }
            Some(Ok(pong_packet)) => pong_packet,
        };
        match pong_packet {
            ProxyControlPacket::TunnelInit(_) => {
                error!("Fail to send heartbeat ping to proxy because of receive invalid control packet from proxy.");
                Err(AgentError::InvalidProxyDataType)
            }
            ProxyControlPacket::Heartbeat(pong) => {
                debug!("Received heartbeat from {pong:?}");
                Ok(())
            }
        }
    }
    async fn fill_pool(
        pool: Arc<Mutex<Vec<PooledProxyConnection<TcpStream>>>>,
        proxy_addresses: Arc<Vec<SocketAddr>>,
        config: Arc<Config>,
        filling_connection: Arc<AtomicBool>,
        initial_pool_size: usize,
    ) {
        if filling_connection.load(Ordering::Acquire) {
            debug!("Filling proxy connection pool, no need to start filling task.");
            return;
        }
        tokio::spawn(async move {
            debug!("Begin to fill proxy connection pool");
            filling_connection.store(true, Ordering::Release);
            let (proxy_connection_tx, mut proxy_connection_rx) =
                channel::<PooledProxyConnection<TcpStream>>(initial_pool_size);
            let current_pool_size = pool.lock().await.len();
            debug!("Current pool size: {current_pool_size}");
            for _ in current_pool_size..initial_pool_size {
                let proxy_addresses = proxy_addresses.clone();
                tokio::spawn(Self::create_proxy_tcp_stream(
                    config.clone(),
                    proxy_addresses,
                    proxy_connection_tx.clone(),
                ));
            }
            drop(proxy_connection_tx);
            debug!("Waiting for proxy connection creation");
            while let Some(proxy_connection) = proxy_connection_rx.recv().await {
                let mut pool = pool.lock().await;
                pool.push(proxy_connection);
                debug!(
                    "Proxy connection creation add to pool, current pool size: {}",
                    pool.len()
                );
            }
            pool.lock()
                .await
                .sort_by(|v1, v2| v1.create_time().cmp(&v2.create_time()));
            filling_connection.store(false, Ordering::Release);
        });
    }
}
