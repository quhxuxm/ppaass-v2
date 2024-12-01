use crate::bo::event::ProxyServerEvent;
use crate::bo::state::ServerState;
use crate::codec::ControlPacketCodec;
use crate::destination::DestinationTransport;
use crate::error::ProxyError;
use crate::publish_server_event;
use futures_util::SinkExt;
use ppaass_crypto::random_32_bytes;
use ppaass_domain::address::UnifiedAddress;
use ppaass_domain::tunnel::{Encryption, TunnelInitRequest, TunnelInitResponse, TunnelType};
use ppaass_domain::ProxyControlPacket;
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, FramedParts};
pub struct TunnelInitResult {
    pub agent_encryption: Encryption,
    pub proxy_encryption: Encryption,
    pub destination_transport: DestinationTransport,
    pub agent_tcp_stream: TcpStream,
    pub destination_address: UnifiedAddress,
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
    let destination_transport = match &tunnel_type {
        TunnelType::Tcp => {
            DestinationTransport::new_tcp(&dst_address, server_state.clone()).await?
        }
        TunnelType::Udp => {
            DestinationTransport::new_udp(&dst_address, server_state.clone()).await?
        }
    };
    let proxy_encryption = Encryption::Aes(random_32_bytes());
    publish_server_event(
        server_state.server_event_tx(),
        ProxyServerEvent::TunnelInit(dst_address.clone()),
    )
    .await;
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
    Ok(TunnelInitResult {
        agent_encryption,
        proxy_encryption,
        destination_transport,
        agent_tcp_stream,
        destination_address: dst_address,
    })
}
