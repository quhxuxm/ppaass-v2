use crate::bo::state::ServerState;
use crate::codec::{ControlPacketCodec, DataPacketCodec};
use crate::error::AgentError;
use crate::pool::PooledProxyConnection;
use bytes::{Bytes, BytesMut};
use futures_util::{SinkExt, StreamExt};
use ppaass_crypto::random_32_bytes;
use ppaass_domain::address::UnifiedAddress;
use ppaass_domain::tunnel::{Encryption, TunnelInitRequest, TunnelInitResponse, TunnelType};
use ppaass_domain::{AgentControlPacket, AgentDataPacket, ProxyControlPacket, ProxyDataPacket};
use tokio::net::TcpStream;
use tokio_stream::StreamExt as TokioStreamExt;
use tokio_util::codec::{BytesCodec, Framed, FramedParts};
use tracing::error;
pub mod http;
pub mod socks5;
pub struct TunnelInitHandlerResponse {
    proxy_tcp_stream: PooledProxyConnection<TcpStream>,
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
    pub proxy_tcp_stream: PooledProxyConnection<TcpStream>,
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
        destination_address,
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
    let client_tcp_framed_rx = client_tcp_framed_rx.map_while(move |client_item| {
        let client_data = match client_item {
            Ok(client_data) => client_data.freeze(),
            Err(e) => {
                error!("Fail to read client data: {e:?}");
                return Some(Err(AgentError::Io(e)));
            }
        };
        Some(Ok(AgentDataPacket::Tcp(client_data.to_vec())))
    });
    let proxy_data_framed_rx = proxy_data_framed_rx.map_while(move |proxy_data_packet| {
        let proxy_packet_data = match proxy_data_packet {
            Ok(proxy_packet_data) => proxy_packet_data,
            Err(e) => {
                error!("Failed to read proxy data: {}", e);
                return Some(Err(e.into()));
            }
        };
        match proxy_packet_data {
            ProxyDataPacket::Tcp(proxy_data) => Some(Ok(BytesMut::from_iter(proxy_data))),
            ProxyDataPacket::Udp {
                destination_address,
                ..
            } => {
                error!("Invalid kind of proxy data, destination address: {destination_address}");
                Some(Err(AgentError::InvalidProxyDataType.into()))
            }
        }
    });
    let (client_to_proxy, proxy_to_client) = futures::join!(
        client_tcp_framed_rx.forward(proxy_data_framed_tx),
        proxy_data_framed_rx.forward(client_tcp_framed_tx)
    );
    if let Err(e) = client_to_proxy {
        error!("Failed to send client data to proxy, destination: [{destination_address}]: {e:?}");
    }
    if let Err(e) = proxy_to_client {
        error!("Failed to send proxy data to client, destination: [{destination_address}]: {e:?}");
    }
    Ok(())
}