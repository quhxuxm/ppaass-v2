use crate::bo::state::ServerState;
use crate::codec::{ControlPacketCodec, DataPacketCodec};
use crate::error::AgentError;
use bytes::{Bytes, BytesMut};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use ppaass_crypto::random_32_bytes;
use ppaass_domain::address::UnifiedAddress;
use ppaass_domain::tunnel::{Encryption, TunnelInitRequest, TunnelInitResponse, TunnelType};
use ppaass_domain::{AgentControlPacket, AgentDataPacket, ProxyControlPacket, ProxyDataPacket};
use tokio::net::TcpStream;
use tokio_util::codec::{BytesCodec, Framed, FramedParts};
use tracing::error;
pub mod http;
pub mod socks5;
type ProxyDataPacketWrite = SplitSink<Framed<TcpStream, DataPacketCodec>, AgentDataPacket>;
type ProxyDataPacketRead = SplitStream<Framed<TcpStream, DataPacketCodec>>;
type ClientDataWrite = SplitSink<Framed<TcpStream, BytesCodec>, BytesMut>;
type ClientDataRead = SplitStream<Framed<TcpStream, BytesCodec>>;
pub struct TunnelInitHandlerResponse {
    proxy_tcp_stream: TcpStream,
    agent_encryption: Encryption,
    proxy_encryption: Encryption,
    destination_address: UnifiedAddress,
}
pub async fn tunnel_init(
    destination_address: UnifiedAddress,
    server_state: ServerState,
) -> Result<TunnelInitHandlerResponse, AgentError> {
    let proxy_tcp_stream = server_state
        .proxy_connection_pool()
        .take_proxy_connection()
        .await?;
    let mut control_framed = Framed::new(
        proxy_tcp_stream,
        ControlPacketCodec::new(
            server_state.config().auth_token().to_owned(),
            server_state.rsa_crypto_holder().clone(),
        ),
    );
    let agent_encryption = Encryption::Aes(random_32_bytes());
    control_framed
        .send(AgentControlPacket::TunnelInit(TunnelInitRequest {
            agent_encryption: agent_encryption.clone(),
            auth_token: server_state.config().auth_token().to_owned(),
            dst_address: destination_address.clone(),
            tunnel_type: TunnelType::Tcp,
        }))
        .await?;
    let TunnelInitResponse { proxy_encryption } = {
        loop {
            let proxy_control_packet = StreamExt::next(&mut control_framed)
                .await
                .ok_or(AgentError::ProxyConnectionExhausted)??;
            match proxy_control_packet {
                ProxyControlPacket::TunnelInit((_, tunnel_init_response)) => {
                    break tunnel_init_response;
                }
                ProxyControlPacket::Heartbeat(heartbeat_pong) => {
                    error!("Receive heartbeat pong from proxy: {:?}", heartbeat_pong);
                    continue;
                }
            }
        }
    };
    let FramedParts {
        io: proxy_tcp_stream,
        ..
    } = control_framed.into_parts();
    Ok(TunnelInitHandlerResponse {
        proxy_tcp_stream,
        agent_encryption,
        proxy_encryption,
        destination_address,
    })
}
pub struct RelayRequest {
    pub client_tcp_stream: TcpStream,
    pub proxy_tcp_stream: TcpStream,
    pub init_data: Option<Bytes>,
    pub agent_encryption: Encryption,
    pub proxy_encryption: Encryption,
    pub destination_address: UnifiedAddress,
}
pub async fn relay(
    relay_request: RelayRequest,
    server_state: ServerState,
) -> Result<(), AgentError> {
    let RelayRequest {
        client_tcp_stream,
        proxy_tcp_stream,
        init_data,
        agent_encryption,
        proxy_encryption,
        destination_address
    } = relay_request;
    let client_tcp_framed = Framed::with_capacity(
        client_tcp_stream,
        BytesCodec::new(),
        *server_state.config().client_relay_buffer_size(),
    );
    let (mut client_tcp_framed_tx, client_tcp_framed_rx) = client_tcp_framed.split::<BytesMut>();
    let proxy_data_framed = Framed::with_capacity(
        proxy_tcp_stream,
        DataPacketCodec::new(agent_encryption, proxy_encryption),
        *server_state.config().proxy_relay_buffer_size(),
    );
    let (proxy_data_framed_tx, proxy_data_framed_rx) = proxy_data_framed.split();
    if let Some(init_data) = init_data {
        client_tcp_framed_tx
            .send(BytesMut::from(init_data.as_ref()))
            .await?;
    }
    let forward_result = futures::join!(  forward_client_to_proxy(&destination_address,client_tcp_framed_rx, proxy_data_framed_tx), forward_proxy_to_client(&destination_address, proxy_data_framed_rx, client_tcp_framed_tx));
    if let (Ok((_, mut proxy_data_packet_write)), Ok((_, mut client_data_write))) = forward_result {
        if let Err(e) = proxy_data_packet_write.close().await {
            error!(destination_address={format!("{destination_address}")}, "Failed to close proxy tcp steam: {e:?}");
        };
        if let Err(e) = client_data_write.close().await {
            error!(destination_address={format!("{destination_address}")}, "Failed to close client tcp steam: {e:?}");
        }
    }
    Ok(())
}
async fn forward_client_to_proxy(destination_address: &UnifiedAddress, mut client_data_read: ClientDataRead, mut proxy_data_packet_write: ProxyDataPacketWrite) -> Result<(ClientDataRead, ProxyDataPacketWrite), AgentError> {
    loop {
        match client_data_read.next().await {
            None => {
                proxy_data_packet_write.flush().await?;
                return Ok((client_data_read, proxy_data_packet_write));
            }
            Some(Err(e)) => {
                error!(destination_address={format!("{destination_address}")},"Fail to forward client data to proxy: {e:?}");
                return Err(e.into());
            }
            Some(Ok(client_data)) => {
                proxy_data_packet_write.send(AgentDataPacket::Tcp(client_data.to_vec())).await?;
            }
        };
    }
}
async fn forward_proxy_to_client(destination_address: &UnifiedAddress, mut proxy_data_read: ProxyDataPacketRead, mut client_data_write: ClientDataWrite) -> Result<(ProxyDataPacketRead, ClientDataWrite), AgentError> {
    loop {
        match proxy_data_read.next().await {
            None => {
                client_data_write.flush().await?;
                return Ok((proxy_data_read, client_data_write));
            }
            Some(Err(e)) => {
                error!(destination_address={format!("{destination_address}")},"Fail to forward destination data to agent: {e:?}");
                return Err(e);
            }
            Some(Ok(proxy_data_packet)) => {
                match proxy_data_packet {
                    ProxyDataPacket::Tcp(data) => {
                        client_data_write.send(BytesMut::from_iter(data)).await?
                    }
                    ProxyDataPacket::Udp { .. } => {
                        return Err(AgentError::Unknown("UDP is not supported".to_owned()))
                    }
                }
            }
        }
    }
}
