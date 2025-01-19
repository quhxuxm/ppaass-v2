use crate::bo::state::{ServerState, ServerStateBuilder};
use crate::codec::ControlPacketCodec;
use crate::config::Config;
use crate::crypto::ProxyRsaCryptoHolder;
use crate::error::ProxyError;
use crate::handler;
use crate::handler::{RelayStartRequest, TunnelInitResult};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use ppaass_domain::heartbeat::HeartbeatPong;
use ppaass_domain::{AgentControlPacket, ProxyControlPacket};
use socket2::{SockRef, TcpKeepalive};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_util::codec::Framed;
use tracing::{debug, error};
const USER_AGENT_PUBLIC_KEY: &str = "AgentPublicKey.pem";
const USER_PROXY_PRIVATE_KEY: &str = "ProxyPrivateKey.pem";
const FORWARD_AGENT_PRIVATE_KEY: &str = "AgentPrivateKey.pem";
const FORWARD_PROXY_PUBLIC_KEY: &str = "ProxyPublicKey.pem";
pub struct ProxyServer {
    server_state: ServerState,
}
impl ProxyServer {
    pub fn new(config: Arc<Config>) -> Result<Self, ProxyError> {
        let mut server_state_builder = ServerStateBuilder::default();
        let mut server_state_builder = server_state_builder
            .config(config.clone())
            .rsa_crypto_holder(Arc::new(ProxyRsaCryptoHolder::new(
                config.rsa_dir(),
                USER_AGENT_PUBLIC_KEY.to_owned(),
                USER_PROXY_PRIVATE_KEY.to_owned(),
            )?));
        if config.forward_server_addresses().is_some() {
            server_state_builder =
                server_state_builder.forward_rsa_crypto_holder(Arc::new(ProxyRsaCryptoHolder::new(
                    config.forward_rsa_dir(),
                    FORWARD_PROXY_PUBLIC_KEY.to_owned(),
                    FORWARD_AGENT_PRIVATE_KEY.to_owned(),
                )?))
        }

        Ok(Self {
            server_state: server_state_builder.build()?,
        })
    }
    fn spawn_agent_task(
        agent_tcp_stream: TcpStream,
        agent_socket_address: SocketAddr,
        server_state: ServerState,
    ) {
        tokio::spawn(async move {
            let mut control_framed = Framed::with_capacity(
                agent_tcp_stream,
                ControlPacketCodec::new(server_state.rsa_crypto_holder().clone()),
                *server_state.config().agent_buffer_size(),
            );
            loop {
                let agent_control_packet = control_framed.next().await;
                match agent_control_packet {
                    None => {
                        debug!(
                            agent_socket_address = { format!("{agent_socket_address}") },
                            "Agent connection exhausted."
                        );
                        return;
                    }
                    Some(Err(e)) => {
                        error!(
                            agent_socket_address = { format!("{agent_socket_address}") },
                            "Fail to receive agent control packet: {:?}", e
                        );
                        return;
                    }
                    Some(Ok(AgentControlPacket::TunnelInit(tunnel_init_request))) => {
                        let tunnel_init_result = match handler::tunnel_init(
                            control_framed,
                            tunnel_init_request,
                            server_state.clone(),
                        )
                        .await
                        {
                            Ok(tunnel_init_result) => tunnel_init_result,
                            Err(e) => {
                                error!(
                                    agent_socket_address = { format!("{agent_socket_address}") },
                                    "Fail to init tunnel: {e:?}"
                                );
                                return;
                            }
                        };
                        match tunnel_init_result {
                            TunnelInitResult::Tcp {
                                agent_encryption,
                                proxy_encryption,
                                destination_tcp_framed,
                                agent_tcp_stream,
                                destination_address,
                            } => {
                                if let Err(e) = handler::start_relay(
                                    agent_tcp_stream,
                                    RelayStartRequest::Tcp {
                                        agent_encryption,
                                        proxy_encryption,
                                        destination_tcp_framed,
                                        destination_address,
                                    },
                                    server_state,
                                )
                                .await
                                {
                                    error!(
                                        agent_socket_address =
                                            { format!("{agent_socket_address}") },
                                        "Fail to start relay tcp data: {e:?}"
                                    );
                                }
                            }
                            TunnelInitResult::Udp {
                                agent_encryption,
                                proxy_encryption,
                                destination_udp_socket,
                                agent_tcp_stream,
                                destination_address,
                            } => {
                                if let Err(e) = handler::start_relay(
                                    agent_tcp_stream,
                                    RelayStartRequest::Udp {
                                        agent_encryption,
                                        proxy_encryption,
                                        destination_udp_socket,
                                        destination_address,
                                    },
                                    server_state,
                                )
                                .await
                                {
                                    error!(
                                        agent_socket_address =
                                            { format!("{agent_socket_address}") },
                                        "Fail to start relay udp data: {e:?}"
                                    );
                                }
                            }
                        }
                        return;
                    }
                    Some(Ok(AgentControlPacket::Heartbeat(heartbeat_ping))) => {
                        debug!(
                            agent_socket_address = { format!("{agent_socket_address}") },
                            "Heartbeat ping received: {:?}", heartbeat_ping
                        );
                        if let Err(e) = control_framed
                            .send(ProxyControlPacket::Heartbeat(HeartbeatPong {
                                heartbeat_time: Utc::now(),
                            }))
                            .await
                        {
                            error!(
                                agent_socket_address = { format!("{agent_socket_address}") },
                                "Fail to send heartbeat pong back to agent: {e:?}"
                            );
                            return;
                        }
                    }
                }
            }
        });
    }
    async fn concrete_start_server(server_state: ServerState) -> Result<(), ProxyError> {
        let server_port = *server_state.config().port();
        let server_socket_addr =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), server_port);
        let server_listener = TcpListener::bind(&server_socket_addr).await?;
        let server_socket = SockRef::from(&server_listener);
        if let Some(buf_size) = server_state.config().agent_socket_receive_buffer_size() {
            server_socket.set_recv_buffer_size(*buf_size)?;
        }
        if let Some(buf_size) = server_state.config().agent_socket_send_buffer_size() {
            server_socket.set_send_buffer_size(*buf_size)?;
        }
        server_socket.set_nodelay(true)?;
        server_socket.set_reuse_address(true)?;
        if *server_state.config().agent_connection_tcp_keepalive() {
            server_socket.set_keepalive(true)?;
            let keepalive = TcpKeepalive::new()
                .with_time(Duration::from_secs(
                    *server_state.config().agent_connection_tcp_keepalive_time(),
                ))
                .with_interval(Duration::from_secs(
                    *server_state
                        .config()
                        .agent_connection_tcp_keepalive_interval(),
                ));
            #[cfg(target_os = "linux")]
            let keepalive = keepalive
                .with_retries(*server_state.config().agent_connection_tcp_keepalive_retry());
            server_socket.set_tcp_keepalive(&keepalive)?;
        }
        server_socket.set_linger(None)?;
        if let Some(read_timeout) = server_state.config().agent_connection_read_timeout() {
            server_socket.set_read_timeout(Some(Duration::from_secs(*read_timeout)))?;
        }
        if let Some(write_timeout) = server_state.config().agent_connection_write_timeout() {
            server_socket.set_write_timeout(Some(Duration::from_secs(*write_timeout)))?;
        }
        loop {
            let (agent_tcp_stream, agent_socket_addr) = server_listener.accept().await?;
            debug!(
                agent_socket_address = { format!("{agent_socket_addr}") },
                "Accept agent tcp connection."
            );
            Self::spawn_agent_task(agent_tcp_stream, agent_socket_addr, server_state.clone());
        }
    }
    pub async fn start(&self) -> Result<JoinHandle<()>, ProxyError> {
        let server_state = self.server_state.clone();
        let handler = tokio::spawn(async move {
            if let Err(e) = Self::concrete_start_server(server_state).await {
                error!("Fail to start server: {e:?}")
            }
        });
        Ok(handler)
    }
}
