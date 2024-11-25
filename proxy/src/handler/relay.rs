use crate::bo::state::ServerState;
use crate::codec::DataPacketCodec;
use crate::destination::{DestinationDataPacket, DestinationTransport};
use crate::error::ProxyError;
use bytes::BytesMut;
use futures_util::StreamExt;
use ppaass_domain::tunnel::Encryption;
use ppaass_domain::{AgentDataPacket, ProxyDataPacket};
use tokio::net::TcpStream;
use tokio_stream::StreamExt as TokioStreamExt;
use tokio_util::codec::Framed;
use tracing::error;
pub struct RelayStartRequest {
    pub agent_encryption: Encryption,
    pub proxy_encryption: Encryption,
    pub destination_transport: DestinationTransport,
}
pub async fn start_relay(
    agent_tcp_stream: TcpStream,
    relay_start_request: RelayStartRequest,
    server_state: ServerState,
) -> Result<(), ProxyError> {
    let RelayStartRequest {
        agent_encryption,
        proxy_encryption,
        destination_transport,
    } = relay_start_request;
    let agent_data_framed = Framed::with_capacity(
        agent_tcp_stream,
        DataPacketCodec::new(agent_encryption, proxy_encryption),
        *server_state.config().agent_buffer_size(),
    );
    let (destination_transport_tx, destination_transport_rx) = destination_transport.split();
    let (agent_data_framed_tx, agent_data_framed_rx) = agent_data_framed.split();
    let agent_data_packet_stream = agent_data_framed_rx.map_while(move |agent_data_packet| {
        let agent_data_packet = match agent_data_packet {
            Ok(agent_data_packet) => agent_data_packet,
            Err(e) => {
                error!("Failed to read agent data: {}", e);
                return Some(Err(e));
            }
        };
        match agent_data_packet {
            AgentDataPacket::Tcp(data) => {
                Some(Ok(BytesMut::from_iter(data)))
            }
            AgentDataPacket::Udp { payload, .. } => {
                Some(Ok(BytesMut::from_iter(payload)))
            }
        }
    });
    let proxy_packet_stream =
        destination_transport_rx.map_while(move |destination_item| {
            let destination_data = match destination_item {
                Ok(destination_data) => destination_data,
                Err(e) => {
                    error!("Failed to read destination data: {e:?}");
                    return Some(Err(e));
                }
            };
            match destination_data {
                DestinationDataPacket::Tcp(data) => Some(Ok(ProxyDataPacket::Tcp(data))),
                DestinationDataPacket::Udp { data, destination_address } => Some(Ok(ProxyDataPacket::Udp {
                    payload: data,
                    destination_address,
                }))
            }
        });
    tokio::spawn(agent_data_packet_stream.forward(destination_transport_tx));
    tokio::spawn(proxy_packet_stream.forward(agent_data_framed_tx));
    Ok(())
}
