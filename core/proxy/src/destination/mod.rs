mod codec;
pub mod read;
pub mod write;
use crate::bo::state::ServerState;
use crate::destination::codec::{
    DestinationDataTcpCodec, ForwardDestinationTransportControlPacketCodec,
};
use crate::destination::read::DestinationTransportRead;
use crate::destination::write::DestinationTransportWrite;
use crate::error::ProxyError;
use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use ppaass_crypto::random_32_bytes;
use ppaass_domain::address::UnifiedAddress;
use ppaass_domain::tunnel::{Encryption, TunnelInitRequest, TunnelInitResponse, TunnelType};
use ppaass_domain::{AgentControlPacket, ProxyControlPacket};
use socket2::{SockRef, TcpKeepalive};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpStream, UdpSocket};
use tokio::time::timeout;
use tokio_util::codec::{Framed, FramedParts};
use tracing::error;
pub enum DestinationDataPacket {
    Tcp(Vec<u8>),
    Udp {
        destination_address: UnifiedAddress,
        data: Vec<u8>,
    },
}
pub enum DestinationTransport {
    Tcp {
        destination_address: UnifiedAddress,
        destination_framed: Framed<TcpStream, DestinationDataTcpCodec>,
    },
    Udp {
        destination_address: UnifiedAddress,
        destination_udp_socket: UdpSocket,
    },
}
impl DestinationTransport {
    pub async fn new_tcp(
        dst_address: &UnifiedAddress,
        keepalive: bool,
        server_state: ServerState,
    ) -> Result<Self, ProxyError> {
        let dst_socket_addresses: Vec<SocketAddr> =
            match server_state.config().forward_server_addresses() {
                None => dst_address.try_into()?,
                Some(forward_addresses) => forward_addresses
                    .iter()
                    .map_while(|addr| {
                        let unified_address: UnifiedAddress = addr.as_str().try_into().ok()?;
                        let socket_addresses: Vec<SocketAddr> = unified_address.try_into().ok()?;
                        Some(socket_addresses)
                    })
                    .flatten()
                    .collect(),
            };
        let dst_tcp_stream = match timeout(
            Duration::from_secs(*server_state.config().dst_connect_timeout()),
            TcpStream::connect(dst_socket_addresses.as_slice()),
        )
            .await
        {
            Ok(Ok(dst_tcp_stream)) => dst_tcp_stream,
            Ok(Err(e)) => {
                error!(
                    dst_addresses = { format!("{dst_socket_addresses:?}") },
                    "Fail to connect destination: {e:?}"
                );
                return Err(e.into());
            }
            Err(e) => {
                error!(
                    dst_addresses = { format!("{dst_socket_addresses:?}") },
                    "Fail to connect destination because of timeout: {} seconds",
                    server_state.config().dst_connect_timeout()
                );
                return Err(e.into());
            }
        };
        let dest_socket = SockRef::from(&dst_tcp_stream);
        dest_socket.set_reuse_address(true)?;
        if keepalive {
            dest_socket.set_keepalive(true)?;
            let keepalive = TcpKeepalive::new()
                .with_time(Duration::from_secs(
                    *server_state.config().dst_tcp_keepalive_time(),
                ))
                .with_interval(Duration::from_secs(
                    *server_state.config().dst_tcp_keepalive_interval(),
                ));
            #[cfg(target_os = "linux")]
            let keepalive =
                keepalive.with_retries(*server_state.config().dst_tcp_keepalive_retry());
            dest_socket.set_tcp_keepalive(&keepalive)?;
        }
        dest_socket.set_nodelay(true)?;
        dest_socket.set_read_timeout(Some(Duration::from_secs(
            *server_state.config().dst_read_timeout(),
        )))?;
        dest_socket.set_write_timeout(Some(Duration::from_secs(
            *server_state.config().dst_write_timeout(),
        )))?;
        dest_socket.set_linger(None)?;
        let destination_framed = match server_state.config().forward_server_addresses() {
            None => Framed::with_capacity(
                dst_tcp_stream,
                DestinationDataTcpCodec::new_raw(),
                *server_state.config().dst_buffer_size(),
            ),
            Some(_) => {
                let forward_auth_token = server_state
                    .config()
                    .forward_auth_token()
                    .clone()
                    .ok_or(ProxyError::InvalidData)?;
                let mut tunnel_init_framed = Framed::with_capacity(
                    dst_tcp_stream,
                    ForwardDestinationTransportControlPacketCodec::new(
                        forward_auth_token.clone(),
                        server_state.forward_rsa_crypto_holder().clone(),
                    ),
                    *server_state.config().dst_buffer_size(),
                );
                let agent_encryption = Encryption::Aes(random_32_bytes());
                let tunnel_init = AgentControlPacket::TunnelInit(TunnelInitRequest {
                    agent_encryption: agent_encryption.clone(),
                    auth_token: forward_auth_token,
                    dst_address: dst_address.clone(),
                    tunnel_type: TunnelType::Tcp { keepalive },
                });
                tunnel_init_framed.send(tunnel_init).await?;
                let TunnelInitResponse { proxy_encryption } = match tunnel_init_framed.next().await
                {
                    None => {
                        return Err(ProxyError::ForwardProxyTcpConnectionExhausted);
                    }
                    Some(Ok(response)) => match response {
                        ProxyControlPacket::TunnelInit((_, tunnel_init_response)) => {
                            tunnel_init_response
                        }
                        ProxyControlPacket::Heartbeat(_) => return Err(ProxyError::InvalidData),
                    },
                    Some(Err(e)) => {
                        return Err(e);
                    }
                };
                let FramedParts {
                    io: dst_tcp_stream, ..
                } = tunnel_init_framed.into_parts();
                Framed::with_capacity(
                    dst_tcp_stream,
                    DestinationDataTcpCodec::new_forward(agent_encryption, proxy_encryption),
                    *server_state.config().dst_buffer_size(),
                )
            }
        };
        Ok(DestinationTransport::Tcp {
            destination_address: dst_address.clone(),
            destination_framed,
        })
    }
    pub async fn new_udp(
        dst_address: &UnifiedAddress,
        _server_state: ServerState,
    ) -> Result<Self, ProxyError> {
        let dst_addresses: Vec<SocketAddr> = dst_address.try_into()?;
        let dst_udp_socket = UdpSocket::bind("0.0.0.0:0").await?;
        dst_udp_socket.connect(dst_addresses.as_slice()).await?;
        Ok(DestinationTransport::Udp {
            destination_address: dst_address.clone(),
            destination_udp_socket: dst_udp_socket,
        })
    }
    // fn convert_addresses(addresses: &[UnifiedAddress]) -> Vec<SocketAddr> {
    //     addresses
    //         .iter()
    //         .map_while(|addr| {
    //             let socket_addresses: Vec<SocketAddr> = addr.try_into().ok()?;
    //             Some(socket_addresses)
    //         })
    //         .flatten()
    //         .collect()
    // }
    pub fn split(self) -> (DestinationTransportWrite, DestinationTransportRead) {
        match self {
            DestinationTransport::Tcp {
                destination_address: _destination_address,
                destination_framed,
            } => {
                let (framed_write, framed_read) = destination_framed.split::<BytesMut>();
                (
                    DestinationTransportWrite::Tcp(framed_write),
                    DestinationTransportRead::Tcp(framed_read),
                )
            }
            DestinationTransport::Udp {
                destination_address,
                destination_udp_socket,
            } => {
                let udp_socket = Arc::new(destination_udp_socket);
                (
                    DestinationTransportWrite::Udp(udp_socket.clone()),
                    DestinationTransportRead::Udp {
                        destination_address,
                        destination_udp_socket: udp_socket,
                    },
                )
            }
        }
    }
}
