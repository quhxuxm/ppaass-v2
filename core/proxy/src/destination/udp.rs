use crate::bo::state::ServerState;
use crate::error::ProxyError;
use ppaass_domain::address::UnifiedAddress;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
pub async fn new_udp_destination(
    dst_address: &UnifiedAddress,
    _server_state: ServerState,
) -> Result<UdpSocket, ProxyError> {
    let dst_addresses: Vec<SocketAddr> = dst_address.try_into()?;
    let dst_udp_socket = UdpSocket::bind("0.0.0.0:0").await?;
    dst_udp_socket.connect(dst_addresses.as_slice()).await?;
    Ok(dst_udp_socket)
}
