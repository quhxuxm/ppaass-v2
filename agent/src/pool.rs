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
        let config_clone = config.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = Self::check_health_and_close(pool_clone.clone(), config_clone.clone(), rsa_crypto_holder.clone()).await {
                    error!("Fail to check pool health: {e}");
                }
                sleep(Duration::from_secs(20)).await;
            }
        });
        Ok(Self { pool, config })
    }
    pub async fn take_proxy_connection(&self) -> Result<TcpStream, AgentError> {
        Self::concrete_take_proxy_connection(self.pool.clone(), self.config.clone()).await
    }
    async fn create_proxy_tcp_stream(config: Arc<Config>, pool: Arc<Mutex<VecDeque<TcpStream>>>) -> Result<(), AgentError> {
        let proxy_socket = TcpSocket::new_v4()?;
        proxy_socket.set_keepalive(true)?;
        proxy_socket.set_reuseaddr(true)?;
        proxy_socket.set_nodelay(true)?;
        let random_index = rand::random::<usize>() % config.proxy_addresses().len();
        let proxy_address = &config.proxy_addresses()[random_index];
        let proxy_socket_addr = SocketAddr::from_str(proxy_address)?;
        let proxy_tcp_stream = proxy_socket.connect(proxy_socket_addr).await?;
        debug!("Create proxy connection on: {proxy_address}");
        let mut pool = pool.lock().await;
        pool.push_back(proxy_tcp_stream);
        Ok(())
    }
    async fn concrete_take_proxy_connection(pool: Arc<Mutex<VecDeque<TcpStream>>>, config: Arc<Config>) -> Result<TcpStream, AgentError> {
        loop {
            let proxy_tcp_stream = pool.lock().await.pop_front();
            match proxy_tcp_stream {
                None => {
                    Self::fill_pool(pool.clone(), config.clone()).await?;
                    continue;
                }
                Some(proxy_tcp_stream) => {
                    return Ok(proxy_tcp_stream)
                }
            }
        }
    }
    async fn check_health_and_close(pool: Arc<Mutex<VecDeque<TcpStream>>>, config: Arc<Config>, rsa_crypto_holder: Arc<AgentRsaCryptoHolder>) -> Result<(), AgentError> {
        let pool_clone = pool.clone();
        let mut pool = pool.lock().await;
        for mut proxy_tcp_stream in pool.drain(..) {
            let config = config.clone();
            let rsa_crypto_holder = rsa_crypto_holder.clone();
            let pool_clone = pool_clone.clone();
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
                        let mut pool_clone = pool_clone.lock().await;
                        pool_clone.push_back(proxy_tcp_stream);
                    }
                }
            });
        }
        Ok(())
    }
    async fn fill_pool(pool: Arc<Mutex<VecDeque<TcpStream>>>, config: Arc<Config>) -> Result<(), AgentError> {
        loop {
            let current_pool_size = {
                let pool = pool.lock().await;
                pool.len()
            };
            if current_pool_size > *config.proxy_connection_pool_size() {
                for _ in *config.proxy_connection_pool_size()..=current_pool_size {
                    pool.lock().await.pop_front();
                }
                return Ok(());
            }
            if current_pool_size == *config.proxy_connection_pool_size() {
                return Ok(());
            }
            debug!("Current pool size: {current_pool_size}");
            tokio::spawn(Self::create_proxy_tcp_stream(config.clone(), pool.clone()));
        }
    }
}
