use crate::bo::config::Config;
use crate::codec::ControlPacketCodec;
use crate::crypto::AgentRsaCryptoHolder;
use crate::error::AgentError;
use crate::pool::{resolve_proxy_address, PooledProxyConnection};
use chrono::Utc;
use concurrent_queue::{ConcurrentQueue, PopError, PushError};
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
use tokio::time::{sleep, timeout};
use tokio_util::codec::{Framed, FramedParts};
use tracing::{debug, error};

/// The connection pool for proxy connection.
pub struct Pooled {
    /// The pool to store the proxy connection
    pool: Arc<ConcurrentQueue<PooledProxyConnection<TcpStream>>>,
    /// The configuration
    config: Arc<Config>,
    /// The proxy addresses
    proxy_addresses: Arc<Vec<SocketAddr>>,
    /// The max pool size
    max_pool_size: usize,
    /// The rsa crypto holder used to store the rsa crypto
    rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
    /// If the filling process is happening
    filling: Arc<AtomicBool>,
}
impl Pooled {
    /// Create the proxy connection pool
    pub async fn new(
        config: Arc<Config>,
        max_pool_size: usize,
        rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
    ) -> Result<Self, AgentError> {
        let proxy_addresses = Arc::new(resolve_proxy_address(&config)?);
        let pool = Arc::new(ConcurrentQueue::bounded(max_pool_size));
        let proxy_addresses = proxy_addresses.clone();
        let filling = Arc::new(AtomicBool::new(false));
        match &config.proxy_connection_pool_fill_interval() {
            None => {
                Self::fill_pool(
                    pool.clone(),
                    proxy_addresses.clone(),
                    config.clone(),
                    max_pool_size,
                    filling.clone(),
                )
                .await;
            }
            Some(interval) => {
                let config = config.clone();
                let interval = *interval;
                let pool = pool.clone();
                let proxy_addresses = proxy_addresses.clone();
                let filling = filling.clone();
                tokio::spawn(async move {
                    loop {
                        debug!("Starting connection pool auto filling loop.");
                        Self::fill_pool(
                            pool.clone(),
                            proxy_addresses.clone(),
                            config.clone(),
                            max_pool_size,
                            filling.clone(),
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
            let pool = pool.clone();
            let filling = filling.clone();
            Self::start_connection_check_task(
                config,
                rsa_crypto_holder,
                pool,
                filling,
                max_pool_size,
            );
        }
        Ok(Self {
            pool,
            config,
            proxy_addresses,
            max_pool_size,
            rsa_crypto_holder,
            filling,
        })
    }

    /// Start the task to check connection activity
    fn start_connection_check_task(
        config: Arc<Config>,
        rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
        pool: Arc<ConcurrentQueue<PooledProxyConnection<TcpStream>>>,
        filling: Arc<AtomicBool>,
        max_pool_size: usize,
    ) {
        tokio::spawn(async move {
            loop {
                if filling.load(Ordering::Relaxed) {
                    debug!("Cancel checking proxy connection pool, because of filling loop is in parallel.");
                    sleep(Duration::from_secs(
                        *config.proxy_connection_start_check_timer_interval(),
                    ))
                    .await;
                    continue;
                }
                debug!(
                    "Start checking connection pool loop, current pool size: {} ",
                    pool.len()
                );
                let (checking_tx, mut checking_rx) =
                    channel::<PooledProxyConnection<TcpStream>>(max_pool_size);
                'checking_single: loop {
                    let proxy_connection = match pool.pop() {
                        Ok(proxy_connection) => proxy_connection,
                        Err(PopError::Closed) => {
                            debug!("Stop checking because of connection pool closed.");
                            return;
                        }
                        Err(PopError::Empty) => {
                            debug!("Stop checking because of connection pool is empty.");
                            break 'checking_single;
                        }
                    };
                    if !proxy_connection.need_check() {
                        if let Err(e) = checking_tx.send(proxy_connection).await {
                            error!("Fail to push proxy connection back to pool: {}", e);
                        }
                        continue 'checking_single;
                    }
                    if proxy_connection.need_close() {
                        debug!("Close proxy connection because of it exceed max life time: {proxy_connection:?}");
                        continue 'checking_single;
                    }
                    let checking_tx = checking_tx.clone();
                    let config = config.clone();
                    let rsa_crypto_holder = rsa_crypto_holder.clone();
                    tokio::spawn(async move {
                        let proxy_connection = match Self::check_proxy_connection(
                            proxy_connection,
                            &config,
                            rsa_crypto_holder.clone(),
                        )
                        .await
                        {
                            Ok(proxy_connection) => proxy_connection,
                            Err(e) => {
                                error!("Failed to check proxy connection: {}", e);
                                return;
                            }
                        };
                        if let Err(e) = checking_tx.send(proxy_connection).await {
                            error!("Fail to push proxy connection back to pool: {}", e);
                        };
                    });
                }
                drop(checking_tx);
                let mut connections = Vec::new();
                while let Some(proxy_connection) = checking_rx.recv().await {
                    connections.push(proxy_connection);
                }
                connections.sort_by(|a, b| a.last_check_time().cmp(b.last_check_time()));
                for proxy_connection in connections {
                    match pool.push(proxy_connection) {
                        Ok(()) => {
                            debug!("Success push proxy connection back to pool after checking, current pool size: {}", pool.len());
                        }
                        Err(PushError::Closed(proxy_connection)) => {
                            debug!("Stop checking because of connection pool closed, current checking proxy connection :{proxy_connection:?}");
                            return;
                        }
                        Err(PushError::Full(proxy_connection)) => {
                            debug!("Drop proxy connection because of after checking connection pool is full, current checking proxy connection :{proxy_connection:?}");
                        }
                    };
                }
                sleep(Duration::from_secs(
                    *config.proxy_connection_start_check_timer_interval(),
                ))
                .await;
            }
        });
    }
    pub async fn take_proxy_connection(
        &self,
    ) -> Result<PooledProxyConnection<TcpStream>, AgentError> {
        Self::concrete_take_proxy_connection(
            self.pool.clone(),
            self.proxy_addresses.clone(),
            self.config.clone(),
            self.max_pool_size,
            self.rsa_crypto_holder.clone(),
            self.filling.clone(),
        )
        .await
    }
    async fn create_proxy_tcp_stream(
        config: Arc<Config>,
        proxy_addresses: Arc<Vec<SocketAddr>>,
        proxy_connection_tx: Sender<PooledProxyConnection<TcpStream>>,
    ) -> Result<(), AgentError> {
        let random_proxy_addr_index = rand::random::<usize>() % proxy_addresses.len();
        let proxy_address = &proxy_addresses[random_proxy_addr_index];
        debug!("Creating proxy tcp stream on: {proxy_address}");
        let proxy_tcp_stream = match timeout(
            Duration::from_secs(*config.proxy_connect_timeout()),
            TcpStream::connect(proxy_address),
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
        proxy_socket.set_linger(None)?;
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

    /// The concrete take proxy connection implementation
    async fn concrete_take_proxy_connection(
        pool: Arc<ConcurrentQueue<PooledProxyConnection<TcpStream>>>,
        proxy_addresses: Arc<Vec<SocketAddr>>,
        config: Arc<Config>,
        pool_size: usize,
        rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
        filling: Arc<AtomicBool>,
    ) -> Result<PooledProxyConnection<TcpStream>, AgentError> {
        loop {
            let pool = pool.clone();
            let current_pool_size = pool.len();
            debug!("Taking proxy connection, current pool size: {current_pool_size}");
            let proxy_connection = pool.pop();
            match proxy_connection {
                Err(PopError::Closed) => {
                    return Err(AgentError::ProxyConnectionPool(
                        "Proxy connection pool closed.".to_string(),
                    ));
                }
                Err(PopError::Empty) => {
                    debug!("No proxy connection available, current pool size: {current_pool_size}");
                    Self::fill_pool(
                        pool,
                        proxy_addresses.clone(),
                        config.clone(),
                        pool_size,
                        filling.clone(),
                    )
                    .await;
                    sleep(Duration::from_secs(
                        *config.proxy_connection_retake_interval(),
                    ))
                    .await;
                    continue;
                }
                Ok(proxy_connection) => {
                    debug!("Proxy connection available, current pool size before take: {current_pool_size}");
                    if !proxy_connection.need_check() {
                        debug!("No need to do proxy connection check: {proxy_connection:?}");
                        return Ok(proxy_connection);
                    } else {
                        match Self::check_proxy_connection(
                            proxy_connection,
                            &config,
                            rsa_crypto_holder.clone(),
                        )
                        .await
                        {
                            Ok(proxy_connection) => return Ok(proxy_connection),
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

    /// Check the proxy connection with sending a ping-pong messasge between agent and proxy
    async fn check_proxy_connection(
        proxy_connection: PooledProxyConnection<TcpStream>,
        config: &Config,
        rsa_crypto_holder: Arc<AgentRsaCryptoHolder>,
    ) -> Result<PooledProxyConnection<TcpStream>, AgentError> {
        debug!("Checking proxy connection : {proxy_connection:?}");
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
        let pong_packet = match timeout(
            Duration::from_secs(*config.proxy_connection_ping_pong_read_timeout()),
            proxy_ctl_framed.next(),
        )
        .await
        {
            Err(_) => {
                error!("Proxy connection do ping pong timeout.");
                return Err(AgentError::ProxyConnectionPingPongTimeout);
            }
            Ok(None) => {
                error!("Proxy connection closed already.");
                return Err(AgentError::ProxyConnectionExhausted);
            }

            Ok(Some(Err(e))) => {
                error!("Fail to receive heartbeat pong from proxy: {e:?}");
                return Err(e);
            }
            Ok(Some(Ok(pong_packet))) => pong_packet,
        };
        match pong_packet {
            ProxyControlPacket::TunnelInit(_) => {
                error!("Fail to send heartbeat ping to proxy because of receive invalid control packet from proxy.");
                Err(AgentError::InvalidProxyDataType)
            }
            ProxyControlPacket::Heartbeat(pong) => {
                debug!("Received heartbeat from {pong:?}");
                let FramedParts {
                    io: mut proxy_connection,
                    ..
                } = proxy_ctl_framed.into_parts();
                proxy_connection.update_check_time();
                Ok(proxy_connection)
            }
        }
    }

    /// Fill the pool with proxy connection
    async fn fill_pool(
        pool: Arc<ConcurrentQueue<PooledProxyConnection<TcpStream>>>,
        proxy_addresses: Arc<Vec<SocketAddr>>,
        config: Arc<Config>,
        max_pool_size: usize,
        filling: Arc<AtomicBool>,
    ) {
        if pool.len() == max_pool_size {
            debug!("Cancel filling proxy connection pool, no need to start filling task(outside task).");
            return;
        }
        tokio::spawn(async move {
            if filling.load(Ordering::Relaxed) {
                debug!(
                    "Cancel filling proxy connection pool, because of filling process is running."
                );
                return;
            }
            if pool.len() == max_pool_size {
                debug!(
                    "Cancel filling proxy connection pool, no need to start filling task(inside task)."
                );
                return;
            }
            debug!("Begin to fill proxy connection pool");
            filling.store(true, Ordering::Relaxed);
            let (proxy_connection_tx, mut proxy_connection_rx) =
                channel::<PooledProxyConnection<TcpStream>>(max_pool_size);
            let current_pool_size = pool.len();
            debug!("Current pool size: {current_pool_size}");
            for _ in current_pool_size..max_pool_size {
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
                match pool.push(proxy_connection) {
                    Ok(()) => {
                        debug!(
                            "Proxy connection creation add to pool, current pool size: {}",
                            pool.len()
                        );
                    }
                    Err(PushError::Full(proxy_connection)) => {
                        error!("Failed to push connection into pool because of pool full: {proxy_connection:?}");
                    }
                    Err(PushError::Closed(proxy_connection)) => {
                        error!("Failed to push connection into pool because of pool closed: {proxy_connection:?}");
                    }
                }
            }
            filling.store(false, Ordering::Relaxed);
        });
    }
}
