use crate::bo::config::Config;
use crate::bo::event::AgentServerEvent;
use crate::bo::state::{ServerState, ServerStateBuilder};
use crate::crypto::AgentRsaCryptoHolder;
use crate::error::AgentError;
use crate::handler::http::handle_http_client_tcp_stream;
use crate::handler::socks5::handle_socks5_client_tcp_stream;
use crate::pool::ProxyConnectionPool;
use crate::publish_server_event;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
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
            _ => handle_http_client_tcp_stream(client_tcp_stream, server_state).await,
        }
    }
    async fn concrete_start_server(server_state: ServerState) -> Result<(), AgentError> {
        let tcp_listener = TcpListener::bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(0, 0, 0, 0),
            *server_state.config().port(),
        )))
        .await?;
        loop {
            let (client_tcp_stream, client_socket_addr) = tcp_listener.accept().await?;
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
        let (server_event_tx, server_event_rx) = channel::<AgentServerEvent>(1024);
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
