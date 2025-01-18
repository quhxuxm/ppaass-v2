use crate::bo::state::ServerState;
use crate::codec::DataPacketCodec;
use crate::destination::DestinationDataTcpCodec;
use crate::error::ProxyError;
use bytes::BytesMut;
use futures_util::StreamExt;
use ppaass_domain::address::UnifiedAddress;
use ppaass_domain::tunnel::Encryption;
use ppaass_domain::{AgentDataPacket, ProxyDataPacket};
use tokio::net::{TcpStream, UdpSocket};
use tokio_stream::StreamExt as TokioStreamExt;
use tokio_util::codec::Framed;
use tracing::error;
pub enum RelayStartRequest {
    Tcp {
        agent_encryption: Encryption,
        proxy_encryption: Encryption,
        destination_tcp_framed: Framed<TcpStream, DestinationDataTcpCodec>,
        destination_address: UnifiedAddress,
    },
    #[allow(unused)]
    Udp {
        agent_encryption: Encryption,
        proxy_encryption: Encryption,
        destination_udp_socket: UdpSocket,
        destination_address: UnifiedAddress,
    },
}
async fn tcp_relay(
    agent_tcp_stream: TcpStream,
    server_state: ServerState,
    agent_encryption: Encryption,
    proxy_encryption: Encryption,
    destination_tcp_framed: Framed<TcpStream, DestinationDataTcpCodec>,
    destination_address: UnifiedAddress,
) -> Result<(), ProxyError> {
    let agent_data_framed = Framed::with_capacity(
        agent_tcp_stream,
        DataPacketCodec::new(agent_encryption, proxy_encryption),
        *server_state.config().agent_buffer_size(),
    );
    let (destination_tcp_framed_tx, destination_tcp_framed_rx) = destination_tcp_framed.split();
    let (agent_data_framed_tx, agent_data_framed_rx) = agent_data_framed.split();
    let destination_address_clone = destination_address.clone();
    let agent_data_framed_rx = agent_data_framed_rx.map_while(move |agent_data_packet| {
        let agent_data_packet = match agent_data_packet {
            Ok(agent_data_packet) => agent_data_packet,
            Err(e) => {
                error!(
                    destination_address = { format!("{destination_address_clone}") },
                    "Failed to read agent data: {}", e
                );
                return Some(Err(e));
            }
        };
        match agent_data_packet {
            AgentDataPacket::Tcp(data) => Some(Ok(BytesMut::from_iter(data))),
            AgentDataPacket::Udp { payload, .. } => Some(Ok(BytesMut::from_iter(payload))),
        }
    });
    let destination_address_clone = destination_address.clone();
    let destination_tcp_framed_rx = destination_tcp_framed_rx.map_while(move |destination_item| {
        let destination_data = match destination_item {
            Ok(destination_data) => destination_data,
            Err(e) => {
                error!(
                    destination_address = { format!("{destination_address_clone}") },
                    "Failed to read destination data: {e:?}"
                );
                return Some(Err(e));
            }
        };
        Some(Ok(ProxyDataPacket::Tcp(destination_data.to_vec())))
    });
    tokio::spawn(agent_data_framed_rx.forward(destination_tcp_framed_tx));
    tokio::spawn(destination_tcp_framed_rx.forward(agent_data_framed_tx));
    Ok(())
}
pub async fn start_relay(
    agent_tcp_stream: TcpStream,
    relay_start_request: RelayStartRequest,
    server_state: ServerState,
) -> Result<(), ProxyError> {
    match relay_start_request {
        RelayStartRequest::Tcp {
            agent_encryption,
            proxy_encryption,
            destination_tcp_framed,
            destination_address,
        } => {
            tcp_relay(
                agent_tcp_stream,
                server_state,
                agent_encryption,
                proxy_encryption,
                destination_tcp_framed,
                destination_address,
            )
            .await
        }
        RelayStartRequest::Udp { .. } => {
            unimplemented!("Udp relay is not implemented yet")
        }
    }
}
