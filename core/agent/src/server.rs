use crate::bo::event::AgentServerEvent;
use crate::bo::state::{ServerState, ServerStateBuilder};
use crate::config::Config;
use crate::crypto::AgentRsaCryptoHolder;
use crate::error::AgentError;
use crate::handler::http::handle_http_client_tcp_stream;
use crate::handler::socks5::handle_socks5_client_tcp_stream;
use crate::pool::ProxyConnectionPool;
use crate::publish_server_event;
use socket2::{SockRef, TcpKeepalive};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{channel, Receiver};
use tracing::{debug, error};
const SOCKS5_VERSION: u8 = 0x05;
const SOCKS4_VERSION: u8 = 0x04;
pub struct AgentServer {
    server_state: ServerState,
}
impl AgentServer {
    pub async fn new(config: Arc<Config>) -> Result<Self, AgentError> {
        let rsa_crypto_holder = Arc::new(AgentRsaCryptoHolder::new(config.clone())?);
        let mut server_state_builder = ServerStateBuilder::default();
        server_state_builder
            .config(config.clone())
            .rsa_crypto_holder(rsa_crypto_holder.clone())
            .proxy_connection_pool(Arc::new(
                ProxyConnectionPool::new(config, rsa_crypto_holder).await?,
            ));
        Ok(Self {
            server_state: server_state_builder.build()?,
        })
    }
    async fn switch_protocol(client_tcp_stream: &TcpStream) -> Result<u8, AgentError> {
        let mut protocol = [0u8; 1];
        client_tcp_stream.peek(&mut protocol).await?;
        if protocol.is_empty() {
            Err(AgentError::ClientTcpConnectionExhausted)
        } else {
            Ok(protocol[0])
        }
    }
    async fn handle_client_tcp_stream(
        client_tcp_stream: TcpStream,
        client_socket_addr: SocketAddr,
        server_state: ServerState,
    ) -> Result<(), AgentError> {
        debug!("Handling client TCP connection: {client_socket_addr}");
        let protocol = Self::switch_protocol(&client_tcp_stream).await?;
        match protocol {
            SOCKS5_VERSION => {
                handle_socks5_client_tcp_stream(client_tcp_stream, server_state).await
            }
            SOCKS4_VERSION => Err(AgentError::UnsupportedSocksV4Protocol),
            b'G' | b'g' | b'H' | b'h' | b'P' | b'p' | b'D' | b'd' | b'C' | b'c' | b'O' | b'o'
            | b'T' | b't' => handle_http_client_tcp_stream(client_tcp_stream, server_state).await,
            protocol => Err(AgentError::Unknown(format!("Unknown protocol: {protocol}"))),
        }
    }
    async fn concrete_start_server(server_state: ServerState) -> Result<(), AgentError> {
        let server_socket_addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            *server_state.config().port(),
        );
        let server_listener = TcpListener::bind(server_socket_addr).await?;
        let server_socket = SockRef::from(&server_listener);
        server_socket.set_nodelay(true)?;
        server_socket.set_reuse_address(true)?;
        if *server_state.config().client_connection_tcp_keepalive() {
            server_socket.set_keepalive(true)?;
            let keepalive = TcpKeepalive::new()
                .with_time(Duration::from_secs(
                    server_state
                        .config()
                        .client_connection_tcp_keepalive_time()
                        .ok_or(AgentError::Unknown(
                            "Client connection tcp keepalive time not given.".to_string(),
                        ))?,
                ))
                .with_interval(Duration::from_secs(
                    server_state
                        .config()
                        .client_connection_tcp_keepalive_interval()
                        .ok_or(AgentError::Unknown(
                            "Client connection tcp keepalive interval not given.".to_string(),
                        ))?,
                ));
            #[cfg(target_os = "linux")]
            let keepalive = keepalive.with_retries(
                *server_state
                    .config()
                    .client_connection_tcp_keepalive_retry(),
            );
            server_socket.set_tcp_keepalive(&keepalive)?;
        }
        server_socket.set_linger(None)?;
        if let Some(buffer_size) = server_state.config().client_socket_receive_buffer_size() {
            server_socket.set_recv_buffer_size(*buffer_size)?;
        }
        if let Some(buffer_size) = server_state.config().client_socket_send_buffer_size() {
            server_socket.set_send_buffer_size(*buffer_size)?;
        }
        if let Some(timeout) = server_state.config().client_connection_read_timeout() {
            server_socket.set_read_timeout(Some(Duration::from_secs(*timeout)))?;
        }
        if let Some(timeout) = server_state.config().client_connection_write_timeout() {
            server_socket.set_write_timeout(Some(Duration::from_secs(*timeout)))?;
        }

        loop {
            let (client_tcp_stream, client_socket_addr) = server_listener.accept().await?;
            let server_state = server_state.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_client_tcp_stream(
                    client_tcp_stream,
                    client_socket_addr,
                    server_state,
                )
                .await
                {
                    error!("Fail to handle client tcp stream [{client_socket_addr:?}]: {e:?}")
                }
            });
        }
    }
    pub async fn start(self) -> Result<Receiver<AgentServerEvent>, AgentError> {
        let (server_event_tx, server_event_rx) =
            channel::<AgentServerEvent>(*self.server_state.config().server_event_max_size());
        {
            let server_event_tx = server_event_tx.clone();
            let server_state = self.server_state.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::concrete_start_server(server_state).await {
                    error!("Fail to start agent server: {e:?}");
                    publish_server_event(server_event_tx, AgentServerEvent::ServerStartFail).await;
                }
            });
        }
        publish_server_event(server_event_tx, AgentServerEvent::ServerStartup).await;
        Ok(server_event_rx)
    }
}
