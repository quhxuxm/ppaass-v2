use crate::bo::state::ServerState;
use crate::destination::codec::{
    DestinationDataTcpCodec, ForwardDestinationTransportControlPacketCodec,
};
use crate::error::ProxyError;
use futures_util::{SinkExt, StreamExt};
use ppaass_crypto::random_32_bytes;
use ppaass_domain::address::UnifiedAddress;
use ppaass_domain::tunnel::{Encryption, TunnelInitRequest, TunnelInitResponse, TunnelType};
use ppaass_domain::{AgentControlPacket, ProxyControlPacket};
use socket2::{SockRef, TcpKeepalive};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::{Framed, FramedParts};
use tracing::error;
pub async fn new_tcp_destination(
    dst_address: &UnifiedAddress,
    keepalive: bool,
    server_state: ServerState,
) -> Result<Framed<TcpStream, DestinationDataTcpCodec>, ProxyError> {
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
    let random_dst_addr_index = rand::random::<usize>() % dst_socket_addresses.len();
    let dst_tcp_stream = match timeout(
        Duration::from_secs(*server_state.config().dst_connect_timeout()),
        TcpStream::connect(&dst_socket_addresses[random_dst_addr_index]),
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
    if let Some(buf_size) = server_state.config().dst_socket_send_buffer_size() {
        dest_socket.set_send_buffer_size(*buf_size)?;
    }
    if let Some(buf_size) = server_state.config().dst_socket_receive_buffer_size() {
        dest_socket.set_recv_buffer_size(*buf_size)?;
    }
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
        let keepalive = keepalive.with_retries(*server_state.config().dst_tcp_keepalive_retry());
        dest_socket.set_tcp_keepalive(&keepalive)?;
    }
    dest_socket.set_nodelay(true)?;
    if let Some(read_timeout) = server_state.config().dst_read_timeout() {
        dest_socket.set_read_timeout(Some(Duration::from_secs(*read_timeout)))?;
    }
    if let Some(write_timeout) = server_state.config().dst_write_timeout() {
        dest_socket.set_write_timeout(Some(Duration::from_secs(*write_timeout)))?;
    }
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
                    server_state.forward_rsa_crypto_holder().clone().ok_or(
                        ProxyError::RsaCryptoNotExist(
                            "Forward proxy rsa crypto holder not initialized".to_string(),
                        ),
                    )?,
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
            let TunnelInitResponse { proxy_encryption } = match tunnel_init_framed.next().await {
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
    Ok(destination_framed)
}
