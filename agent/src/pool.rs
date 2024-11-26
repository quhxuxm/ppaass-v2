use crate::bo::config::Config;
use crate::codec::ControlPacketCodec;
use crate::crypto::AgentRsaCryptoHolder;
use crate::error::AgentError;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use ppaass_domain::heartbeat::HeartbeatPing;
use ppaass_domain::{AgentControlPacket, ProxyControlPacket};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_util::codec::Framed;
use tracing::{debug, error};
pub struct ProxyConnectionPool {
    pool: Arc<Mutex<VecDeque<TcpStream>>>,
    config: Arc<Config>,
    proxy_addresses: Arc<Vec<SocketAddr>>,
    filling_connection: Arc<AtomicBool>,
}
impl ProxyConnectionPool {
    pub async fn new(
        config: Arc<Config>,
        rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
    ) -> Result<Self, AgentError> {
        let pool = Arc::new(Mutex::new(VecDeque::with_capacity(
            *config.proxy_connection_pool_size(),
        )));
        let pool_clone = pool.clone();
        let proxy_addresses = Arc::new(
            config
                .proxy_addresses()
                .iter()
                .filter_map(|addr| SocketAddr::from_str(addr).ok())
                .collect::<Vec<SocketAddr>>(),
        );
        let filling_connection = Arc::new(AtomicBool::new(false));
        tokio::spawn(Self::check_health_and_close(
            pool_clone.clone(),
            config.clone(),
            rsa_crypto_holder.clone(),
        ));
        tokio::spawn(Self::fill_pool(
            pool.clone(),
            proxy_addresses.clone(),
            config.clone(),
            filling_connection.clone(),
        ));
        Ok(Self {
            pool,
            config,
            proxy_addresses,
            filling_connection,
        })
    }
    pub async fn take_proxy_connection(&self) -> Result<TcpStream, AgentError> {
        Self::concrete_take_proxy_connection(
            self.pool.clone(),
            self.proxy_addresses.clone(),
            self.config.clone(),
            self.filling_connection.clone(),
        )
        .await
    }
    pub async fn return_proxy_connection(
        &self,
        proxy_tcp_stream: TcpStream,
    ) -> Result<(), AgentError> {
        let mut pool = self.pool.lock().await;
        if pool.len() >= *self.config.proxy_connection_pool_size() {
            return Ok(());
        }
        pool.push_back(proxy_tcp_stream);
        Ok(())
    }
    async fn create_proxy_tcp_stream(
        _config: Arc<Config>,
        proxy_addresses: Arc<Vec<SocketAddr>>,
        proxy_connection_tx: Sender<TcpStream>,
    ) -> Result<(), AgentError> {
        let proxy_tcp_stream = TcpStream::connect(proxy_addresses.as_slice()).await?;
        debug!("Create proxy connection: {proxy_tcp_stream:?}");
        proxy_connection_tx
            .send(proxy_tcp_stream)
            .await
            .map_err(|_| {
                AgentError::ProxyConnectionPool("Fail to send proxy connection".to_string())
            })?;
        Ok(())
    }
    async fn concrete_take_proxy_connection(
        pool: Arc<Mutex<VecDeque<TcpStream>>>,
        proxy_addresses: Arc<Vec<SocketAddr>>,
        config: Arc<Config>,
        filling_connection: Arc<AtomicBool>,
    ) -> Result<TcpStream, AgentError> {
        loop {
            let pool_clone = pool.clone();
            let mut pool = pool.lock().await;
            let current_pool_size = pool.len();
            debug!("Taking proxy connection, current pool size: {current_pool_size}");
            let proxy_tcp_stream = pool.pop_front();
            drop(pool);
            match proxy_tcp_stream {
                None => {
                    debug!("No proxy connection available, current pool size: {current_pool_size}");
                    Self::fill_pool(
                        pool_clone,
                        proxy_addresses.clone(),
                        config.clone(),
                        filling_connection.clone(),
                    )
                    .await?;
                    continue;
                }
                Some(proxy_tcp_stream) => {
                    debug!("Proxy connection available, current pool size before take: {current_pool_size}");
                    return Ok(proxy_tcp_stream);
                }
            }
        }
    }
    async fn check_health_and_close(
        pool: Arc<Mutex<VecDeque<TcpStream>>>,
        config: Arc<Config>,
        rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
    ) -> Result<(), AgentError> {
        loop {
            sleep(Duration::from_secs(
                *config.proxy_connection_heartbeat_interval(),
            ))
            .await;
            debug!("Begin proxy connection health check");
            let (proxy_conn_tx, mut proxy_conn_rx) = channel::<TcpStream>(1024);
            let mut pool = pool.lock().await;
            for mut proxy_tcp_stream in pool.drain(..) {
                debug!("Checking proxy connection from: {proxy_tcp_stream:?}");
                let config = config.clone();
                let rsa_crypto_holder = rsa_crypto_holder.clone();
                let proxy_conn_tx = proxy_conn_tx.clone();
                tokio::spawn(async move {
                    let mut proxy_ctl_framed = Framed::new(
                        &mut proxy_tcp_stream,
                        ControlPacketCodec::new(
                            config.auth_token().to_owned(),
                            rsa_crypto_holder.clone(),
                        ),
                    );
                    if let Err(e) = proxy_ctl_framed
                        .send(AgentControlPacket::Heartbeat(HeartbeatPing {
                            heartbeat_time: Utc::now(),
                        }))
                        .await
                    {
                        error!("Fail to send heartbeat ping to proxy: {e}");
                        return;
                    };
                    let pong_packet = match proxy_ctl_framed.next().await {
                        None => {
                            error!("Proxy connection closed already.");
                            return;
                        }
                        Some(Err(e)) => {
                            error!("Fail to receive heartbeat pong from proxy: {e:?}");
                            return;
                        }
                        Some(Ok(pong_packet)) => pong_packet,
                    };
                    match pong_packet {
                        ProxyControlPacket::TunnelInit(_) => {
                            error!("Fail to send heartbeat ping to proxy because of receive invalid control packet from proxy.");
                        }
                        ProxyControlPacket::Heartbeat(pong) => {
                            debug!("Received heartbeat from {pong:?}");
                            if let Err(e) = proxy_conn_tx.send(proxy_tcp_stream).await {
                                error!("Fail to send proxy connection: {e}");
                            };
                        }
                    }
                });
            }
            drop(proxy_conn_tx);
            debug!("Health check waiting for proxy connection back to pool.");
            while let Some(proxy_tcp_stream) = proxy_conn_rx.recv().await {
                pool.push_back(proxy_tcp_stream);
                debug!(
                    "Health check push proxy connection back to pool, current pool size: {}",
                    pool.len()
                );
            }
        }
    }
    async fn fill_pool(
        pool: Arc<Mutex<VecDeque<TcpStream>>>,
        proxy_addresses: Arc<Vec<SocketAddr>>,
        config: Arc<Config>,
        filling_connection: Arc<AtomicBool>,
    ) -> Result<(), AgentError> {
        debug!("Begin to fill proxy connection pool");
        if filling_connection.load(Ordering::Acquire) {
            return Ok(());
        }
        filling_connection.store(true, Ordering::Release);
        let (proxy_connection_tx, mut proxy_connection_rx) = channel::<TcpStream>(1024);
        let mut pool = pool.lock().await;
        let current_pool_size = pool.len();
        debug!("Current pool size: {current_pool_size}");
        for _ in current_pool_size..*config.proxy_connection_pool_size() {
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
            pool.push_back(proxy_connection);
            debug!(
                "Proxy connection creation add to pool, current pool size: {}",
                pool.len()
            );
        }
        filling_connection.store(false, Ordering::Release);
        Ok(())
    }
}
