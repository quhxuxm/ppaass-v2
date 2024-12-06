use crate::bo::config::Config;
use crate::codec::ControlPacketCodec;
use crate::crypto::AgentRsaCryptoHolder;
use crate::error::AgentError;
use crate::pool::{parse_proxy_address, PooledProxyConnection};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use ppaass_domain::heartbeat::HeartbeatPing;
use ppaass_domain::{AgentControlPacket, ProxyControlPacket};
use socket2::{SockRef, TcpKeepalive};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};
use tokio_util::codec::{Framed, FramedParts};
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
            let filling_connection = filling_connection.clone();
            match &config.proxy_connection_pool_fill_interval() {
                None => {
                    Self::fill_pool(
                        pool.clone(),
                        proxy_addresses.clone(),
                        config.clone(),
                        filling_connection.clone(),
                        initial_pool_size,
                    )
                        .await;
                }
                Some(interval) => {
                    let config = config.clone();
                    let interval = *interval;
                    let pool = pool.clone();
                    tokio::spawn(async move {
                        loop {
                            debug!("Starting connection pool auto filling loop.");
                            Self::fill_pool(
                                pool.clone(),
                                proxy_addresses.clone(),
                                config.clone(),
                                filling_connection.clone(),
                                initial_pool_size,
                            )
                                .await;
                            sleep(Duration::from_secs(interval)).await;
                        }
                    });
                }
            }
            if *config.proxy_connection_start_check_timer() {
                let config = config.clone();
                let rsa_crypto_holder = rsa_crypto_holder.clone();
                tokio::spawn(async move {
                    loop {
                        debug!("Start checking connection pool loop.");
                        {
                            let mut remove_indexes = vec![];
                            let mut pool = pool.lock().await;
                            for (index, proxy_connection) in pool.iter_mut().enumerate() {
                                if !proxy_connection.need_check() {
                                    continue;
                                }
                                if let Err(e) = Self::check_proxy_connection(proxy_connection, config.clone(), rsa_crypto_holder.clone()).await {
                                    error!("Failed to check proxy connection: {}", e);
                                    remove_indexes.push(index);
                                };
                            }
                            for index in remove_indexes {
                                pool.remove(index);
                            }
                        }
                        sleep(Duration::from_secs(*config.proxy_connection_start_check_timer_interval())).await;
                    }
                });
            }
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
        let proxy_tcp_stream = match timeout(
            Duration::from_secs(*config.proxy_connect_timeout()),
            TcpStream::connect(proxy_addresses.as_slice()),
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
                    *config.proxy_connect_timeout()
                );
                return Err(e.into());
            }
        };
        let proxy_socket = SockRef::from(&proxy_tcp_stream);
        proxy_socket.set_reuse_address(true)?;
        proxy_socket.set_keepalive(true)?;
        let keepalive = TcpKeepalive::new()
            .with_interval(Duration::from_secs(
                *config.proxy_connection_tcp_keepalive_interval(),
            ))
            .with_time(Duration::from_secs(
                *config.proxy_connection_tcp_keepalive_time(),
            ));
        #[cfg(target_os = "linux")]
        keepalive.with_retries(*config.proxy_connection_tcp_keepalive_retry());
        proxy_socket.set_tcp_keepalive(&keepalive)?;
        proxy_socket.set_nodelay(true)?;
        proxy_socket.set_read_timeout(Some(Duration::from_secs(
            *config.proxy_connection_read_timeout(),
        )))?;
        proxy_socket.set_write_timeout(Some(Duration::from_secs(
            *config.proxy_connection_write_timeout(),
        )))?;
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
                let FramedParts { io: proxy_connection, .. } = proxy_ctl_framed.into_parts();
                proxy_connection.update_check_time();
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
        if filling_connection.load(Ordering::Relaxed) {
            debug!("Filling proxy connection pool, no need to start filling task(outside task).");
            return;
        }
        tokio::spawn(async move {
            if filling_connection.load(Ordering::Relaxed) {
                debug!(
                    "Filling proxy connection pool, no need to start filling task(inside task)."
                );
                return;
            }
            debug!("Begin to fill proxy connection pool");
            filling_connection.store(true, Ordering::Relaxed);
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
                .sort_by(|v1, v2| v1.last_check_time().cmp(v2.last_check_time()));
            filling_connection.store(false, Ordering::Relaxed);
        });
    }
}
