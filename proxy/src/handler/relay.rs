use crate::bo::state::ServerState;
use crate::codec::DataPacketCodec;
use crate::destination::read::DestinationTransportRead;
use crate::destination::write::DestinationTransportWrite;
use crate::destination::{DestinationDataPacket, DestinationTransport};
use crate::error::ProxyError;
use bytes::BytesMut;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use ppaass_domain::address::UnifiedAddress;
use ppaass_domain::tunnel::Encryption;
use ppaass_domain::{AgentDataPacket, ProxyDataPacket};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::error;
type AgentDataPacketWrite = SplitSink<Framed<TcpStream, DataPacketCodec>, ProxyDataPacket>;
type AgentDataPacketRead = SplitStream<Framed<TcpStream, DataPacketCodec>>;
pub struct RelayStartRequest {
    pub agent_encryption: Encryption,
    pub proxy_encryption: Encryption,
    pub destination_transport: DestinationTransport,
    pub destination_address: UnifiedAddress,
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
        destination_address
    } = relay_start_request;
    let agent_data_framed = Framed::with_capacity(
        agent_tcp_stream,
        DataPacketCodec::new(agent_encryption, proxy_encryption),
        *server_state.config().agent_buffer_size(),
    );
    let (destination_transport_tx, destination_transport_rx) = destination_transport.split();
    let (agent_data_framed_tx, agent_data_framed_rx) = agent_data_framed.split();
    let forward_result = futures::join!(forward_agent_to_destination(&destination_address,agent_data_framed_rx,destination_transport_tx), forward_destination_to_agent(&destination_address, destination_transport_rx,agent_data_framed_tx));
    if let (Ok(mut destination_transport_tx), Ok(mut agent_data_framed_tx)) = forward_result {
        if let Err(e) = agent_data_framed_tx.close().await {
            error!(destination_address={format!("{destination_address}")}, "Failed to close agent tcp steam: {e:?}");
        };
        if let Err(e) = destination_transport_tx.close().await {
            error!(destination_address={format!("{destination_address}")}, "Failed to close destination tcp steam: {e:?}");
        }
    }
    Ok(())
}
async fn forward_agent_to_destination(destination_address: &UnifiedAddress, mut agent_data_framed_rx: AgentDataPacketRead, mut destination_transport_tx: DestinationTransportWrite) -> Result<DestinationTransportWrite, ProxyError> {
    loop {
        match agent_data_framed_rx.next().await {
            None => {
                destination_transport_tx.flush().await?;
                return Ok(destination_transport_tx);
            }
            Some(Err(e)) => {
                error!(destination_address={format!("{destination_address}")},"Fail to forward agent data to destination: {e:?}");
                destination_transport_tx.close().await?;
                return Err(e);
            }
            Some(Ok(agent_data_packet)) => {
                match agent_data_packet {
                    AgentDataPacket::Tcp(data) => {
                        if let Err(e) = destination_transport_tx.send(BytesMut::from_iter(data)).await {
                            destination_transport_tx.close().await?;
                            error!("Failed to forward agent data to destination: {e:?}");
                            return Err(e);
                        }
                    }
                    AgentDataPacket::Udp { payload, .. } => {
                        if let Err(e) = destination_transport_tx.send(BytesMut::from_iter(payload)).await {
                            destination_transport_tx.close().await?;
                            error!("Failed to forward destination data to agent: {e:?}");
                            return Err(e);
                        }
                    }
                }
            }
        };
    }
}
async fn forward_destination_to_agent(destination_address: &UnifiedAddress, mut destination_transport_rx: DestinationTransportRead, mut agent_data_framed_tx: AgentDataPacketWrite) -> Result<AgentDataPacketWrite, ProxyError> {
    loop {
        match destination_transport_rx.next().await {
            None => {
                agent_data_framed_tx.flush().await?;
                return Ok(agent_data_framed_tx);
            }
            Some(Err(e)) => {
                error!(destination_address={format!("{destination_address}")},"Fail to forward destination data to agent: {e:?}");
                agent_data_framed_tx.close().await?;
                return Err(e);
            }
            Some(Ok(destination_data_packet)) => {
                match destination_data_packet {
                    DestinationDataPacket::Tcp(data) => {
                        if let Err(e) = agent_data_framed_tx.send(ProxyDataPacket::Tcp(data)).await {
                            agent_data_framed_tx.close().await?;
                            error!("Fail to forward destination tcp data to agent: {e:?}");
                            return Err(e);
                        }
                    }
                    DestinationDataPacket::Udp { data, destination_address } => {
                        if let Err(e) = agent_data_framed_tx.send(ProxyDataPacket::Udp {
                            payload: data,
                            destination_address,
                        }).await {
                            agent_data_framed_tx.close().await?;
                            error!("Fail to forward destination udp data to agent: {e:?}");
                            return Err(e);
                        }
                    }
                }
            }
        }
    }
}
