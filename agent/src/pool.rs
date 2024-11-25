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
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpSocket, TcpStream};
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_util::codec::Framed;
use tracing::{debug, error};
pub struct ProxyConnectionPool {
    pool: Arc<Mutex<VecDeque<TcpStream>>>,
    config: Arc<Config>,
}
impl ProxyConnectionPool {
    pub async fn new(config: Arc<Config>, rsa_crypto_holder: Arc<AgentRsaCryptoHolder>) -> Result<Self, AgentError> {
        let pool = Arc::new(Mutex::new(VecDeque::with_capacity(*config.proxy_connection_pool_size())));
        let pool_clone = pool.clone();
        tokio::spawn(Self::check_health_and_close(pool_clone.clone(), config.clone(), rsa_crypto_holder.clone()));
        tokio::spawn(Self::fill_pool(pool.clone(), config.clone()));
        Ok(Self { pool, config })
    }
    pub async fn take_proxy_connection(&self) -> Result<TcpStream, AgentError> {
        Self::concrete_take_proxy_connection(self.pool.clone(), self.config.clone()).await
    }
    async fn create_proxy_tcp_stream(config: Arc<Config>, proxy_connection_tx: Sender<TcpStream>) -> Result<(), AgentError> {
        let proxy_socket = TcpSocket::new_v4()?;
        proxy_socket.set_keepalive(true)?;
        proxy_socket.set_reuseaddr(true)?;
        proxy_socket.set_nodelay(true)?;
        let random_index = rand::random::<usize>() % config.proxy_addresses().len();
        let proxy_address = &config.proxy_addresses()[random_index];
        let proxy_socket_addr = SocketAddr::from_str(proxy_address)?;
        let proxy_tcp_stream = proxy_socket.connect(proxy_socket_addr).await?;
        debug!("Create proxy connection on: {proxy_address}");
        proxy_connection_tx.send(proxy_tcp_stream).await.map_err(|_| AgentError::ProxyConnectionPool("Fail to send proxy connection".to_string()))?;
        Ok(())
    }
    async fn concrete_take_proxy_connection(pool: Arc<Mutex<VecDeque<TcpStream>>>, _config: Arc<Config>) -> Result<TcpStream, AgentError> {
        loop {
            debug!("Taking proxy connection");
            let mut pool = pool.lock().await;
            let proxy_tcp_stream = pool.pop_front();
            match proxy_tcp_stream {
                None => {
                    debug!("No proxy connection available, current pool size: {}", pool.len());
                    continue;
                }
                Some(proxy_tcp_stream) => {
                    debug!("Proxy connection available, current pool size: {}", pool.len());
                    return Ok(proxy_tcp_stream);
                }
            }
        }
    }
    async fn check_health_and_close(pool: Arc<Mutex<VecDeque<TcpStream>>>, config: Arc<Config>, rsa_crypto_holder: Arc<AgentRsaCryptoHolder>) -> Result<(), AgentError> {
        loop {
            debug!("Begin proxy connection health check");
            {
                let (proxy_conn_tx, mut proxy_conn_rx) = channel::<TcpStream>(1024);
                let mut pool = pool.lock().await;
                for mut proxy_tcp_stream in pool.drain(..) {
                    debug!("Checking proxy connection from: {proxy_tcp_stream:?}");
                    let config = config.clone();
                    let rsa_crypto_holder = rsa_crypto_holder.clone();
                    let proxy_conn_tx = proxy_conn_tx.clone();
                    tokio::spawn(async move {
                        let mut proxy_ctl_framed = Framed::new(&mut proxy_tcp_stream, ControlPacketCodec::new(config.auth_token().to_owned(), rsa_crypto_holder.clone()));
                        if let Err(e) = proxy_ctl_framed.send(AgentControlPacket::Heartbeat(HeartbeatPing {
                            heartbeat_time: Utc::now()
                        })).await {
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
                    debug!("Health check push proxy connection back to pool, current pool size: {}", pool.len());
                }
            }
            sleep(Duration::from_secs(20)).await;
        }
    }
    async fn fill_pool(pool: Arc<Mutex<VecDeque<TcpStream>>>, config: Arc<Config>) -> Result<(), AgentError> {
        debug!("Begin to fill proxy connection pool");
        loop {
            {
                let (proxy_connection_tx, mut proxy_connection_rx) = channel::<TcpStream>(1024);
                let mut pool = pool.lock().await;
                let current_pool_size = pool.len();
                debug!("Current pool size: {current_pool_size}");
                for _ in current_pool_size..*config.proxy_connection_pool_size() {
                    tokio::spawn(Self::create_proxy_tcp_stream(config.clone(), proxy_connection_tx.clone()));
                }
                drop(proxy_connection_tx);
                debug!("Waiting for proxy connection creation");
                while let Some(proxy_connection) = proxy_connection_rx.recv().await {
                    pool.push_back(proxy_connection);
                    debug!("Proxy connection creation add to pool, current pool size: {}",pool.len());
                }
            }
            sleep(Duration::from_secs(2)).await;
        }
    }
}
