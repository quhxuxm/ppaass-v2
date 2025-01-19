use crate::bo::state::ServerState;
use crate::codec::ControlPacketCodec;
use crate::destination::{new_tcp_destination, new_udp_destination, DestinationDataTcpCodec};
use crate::error::ProxyError;
use futures_util::SinkExt;
use ppaass_crypto::random_32_bytes;
use ppaass_domain::address::UnifiedAddress;
use ppaass_domain::tunnel::{Encryption, TunnelInitRequest, TunnelInitResponse, TunnelType};
use ppaass_domain::ProxyControlPacket;
use tokio::net::{TcpStream, UdpSocket};
use tokio_util::codec::{Framed, FramedParts};
pub enum TunnelInitResult {
    Tcp {
        agent_encryption: Encryption,
        proxy_encryption: Encryption,
        destination_tcp_framed: Framed<TcpStream, DestinationDataTcpCodec>,
        agent_tcp_stream: TcpStream,
        destination_address: UnifiedAddress,
    },
    Udp {
        agent_encryption: Encryption,
        proxy_encryption: Encryption,
        destination_udp_socket: UdpSocket,
        agent_tcp_stream: TcpStream,
        destination_address: UnifiedAddress,
    },
}
/// Create tunnel in proxy side
pub async fn tunnel_init(
    mut agent_control_framed: Framed<TcpStream, ControlPacketCodec>,
    tunnel_init_request: TunnelInitRequest,
    server_state: ServerState,
) -> Result<TunnelInitResult, ProxyError> {
    let TunnelInitRequest {
        agent_encryption,
        auth_token,
        dst_address,
        tunnel_type,
    } = tunnel_init_request;
    match &tunnel_type {
        TunnelType::Tcp { keepalive } => {
            let destination_tcp_framed =
                new_tcp_destination(&dst_address, *keepalive, server_state.clone()).await?;
            let proxy_encryption = Encryption::Aes(random_32_bytes());

            let tunnel_init_response = TunnelInitResponse {
                proxy_encryption: proxy_encryption.clone(),
            };
            let proxy_control_packet =
                ProxyControlPacket::TunnelInit((auth_token.clone(), tunnel_init_response));
            agent_control_framed.send(proxy_control_packet).await?;
            let FramedParts {
                io: agent_tcp_stream,
                ..
            } = agent_control_framed.into_parts();
            Ok(TunnelInitResult::Tcp {
                agent_encryption,
                proxy_encryption,
                destination_tcp_framed,
                agent_tcp_stream,
                destination_address: dst_address,
            })
        }
        TunnelType::Udp => {
            let destination_udp_socket =
                new_udp_destination(&dst_address, server_state.clone()).await?;
            let proxy_encryption = Encryption::Aes(random_32_bytes());
            let tunnel_init_response = TunnelInitResponse {
                proxy_encryption: proxy_encryption.clone(),
            };
            let proxy_control_packet =
                ProxyControlPacket::TunnelInit((auth_token.clone(), tunnel_init_response));
            agent_control_framed.send(proxy_control_packet).await?;
            let FramedParts {
                io: agent_tcp_stream,
                ..
            } = agent_control_framed.into_parts();
            Ok(TunnelInitResult::Udp {
                agent_encryption,
                proxy_encryption,
                destination_udp_socket,
                agent_tcp_stream,
                destination_address: dst_address,
            })
        }
    }
}
