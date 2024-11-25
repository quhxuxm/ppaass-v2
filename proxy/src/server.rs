use crate::bo::config::Config;
use crate::bo::event::ProxyServerEvent;
use crate::bo::state::{ServerState, ServerStateBuilder};
use crate::codec::ControlPacketCodec;
use crate::crypto::ProxyRsaCryptoHolder;
use crate::error::ProxyError;
use crate::handler::{RelayStartRequest, TunnelInitResult};
use crate::{handler, publish_server_event};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use ppaass_domain::heartbeat::HeartbeatPong;
use ppaass_domain::{AgentControlPacket, ProxyControlPacket};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpSocket, TcpStream};
use tokio::sync::mpsc::{channel, Receiver};
use tokio_util::codec::Framed;
use tracing::{debug, error};
pub struct ProxyServer {
    server_state: ServerState,
}
impl ProxyServer {
    pub fn new(config: Arc<Config>) -> Result<(Self, Receiver<ProxyServerEvent>), ProxyError> {
        let (server_event_tx, server_event_rx) = channel::<ProxyServerEvent>(1024);
        let mut server_state_builder = ServerStateBuilder::default();
        server_state_builder
            .config(config.clone())
            .rsa_crypto_holder(Arc::new(ProxyRsaCryptoHolder::new(config)?))
            .server_event_tx(Arc::new(server_event_tx));
        Ok((
            Self {
                server_state: server_state_builder.build()?,
            },
            server_event_rx,
        ))
    }
    fn spawn_agent_task(agent_tcp_stream: TcpStream, server_state: ServerState) {
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
                        return;
                    }
                    Some(Err(e)) => {
                        error!("Fail to receive agent control packet: {:?}", e);
                        return;
                    }
                    Some(Ok(AgentControlPacket::TunnelInit(tunnel_init_request))) => {
                        let TunnelInitResult {
                            agent_encryption,
                            proxy_encryption,
                            destination_transport,
                            agent_tcp_stream,
                        } = match handler::tunnel_init(control_framed, tunnel_init_request, server_state.clone()).await {
                            Ok(tunnel_init_result) => tunnel_init_result,
                            Err(e) => {
                                error!("Fail to init tunnel: {e:?}");
                                return;
                            }
                        };
                        if let Err(e) = handler::start_relay(
                            agent_tcp_stream,
                            RelayStartRequest {
                                agent_encryption,
                                proxy_encryption,
                                destination_transport,
                            },
                            server_state,
                        )
                            .await
                        {
                            error!("Fail to start relay: {e:?}");
                        }
                        return;
                    }
                    Some(Ok(AgentControlPacket::Heartbeat(heartbeat_ping))) => {
                        debug!("Heartbeat ping received: {:?}", heartbeat_ping);
                        if let Err(e) = control_framed.send(ProxyControlPacket::Heartbeat(HeartbeatPong {
                            heartbeat_time: Utc::now(),
                        })).await {
                            error!("Fail to send heartbeat pong back to agent: {e:?}");
                            return;
                        }
                    }
                }
            }
        });
    }
    async fn concrete_start_server(server_state: ServerState) -> Result<(), ProxyError> {
        let server_port = *server_state.config().port();
        let server_socket = TcpSocket::new_v4()?;
        server_socket.set_keepalive(true)?;
        server_socket.set_reuseaddr(true)?;
        server_socket
            .set_recv_buffer_size(*server_state.config().server_socket_recv_buffer_size())?;
        server_socket
            .set_send_buffer_size(*server_state.config().server_socket_send_buffer_size())?;
        server_socket.set_nodelay(true)?;
        server_socket.bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            server_port,
        ))?;
        let server_listener = server_socket.listen(1024)?;
        loop {
            let (agent_tcp_stream, agent_socket_addr) = server_listener.accept().await?;
            debug!("Accept agent tcp connection from: {agent_socket_addr}");
            Self::spawn_agent_task(agent_tcp_stream, server_state.clone());
        }
    }
    pub async fn start(
        &self,
        server_event_rx: Receiver<ProxyServerEvent>,
    ) -> Result<Receiver<ProxyServerEvent>, ProxyError> {
        {
            let server_state = self.server_state.clone();
            let server_event_tx_clone = server_state.server_event_tx().clone();
            tokio::spawn(async move {
                if let Err(e) = Self::concrete_start_server(server_state).await {
                    publish_server_event(&server_event_tx_clone, ProxyServerEvent::ServerStartFail)
                        .await;
                    error!("Fail to start server: {e:?}")
                }
            });
        }
        publish_server_event(
            self.server_state.server_event_tx(),
            ProxyServerEvent::ServerStartup,
        )
            .await;
        Ok(server_event_rx)
    }
}
