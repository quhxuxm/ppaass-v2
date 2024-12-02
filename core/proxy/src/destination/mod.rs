mod codec;
pub mod read;
pub mod write;
use crate::bo::state::ServerState;
use crate::destination::codec::DestinationDataTcpCodec;
use crate::destination::read::DestinationTransportRead;
use crate::destination::write::DestinationTransportWrite;
use crate::error::ProxyError;
use bytes::BytesMut;
use futures_util::StreamExt;
use ppaass_domain::address::UnifiedAddress;
use rand::random;
use socket2::{Domain, Protocol, Socket, TcpKeepalive, Type};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpStream, UdpSocket};
use tokio_util::codec::Framed;
pub enum DestinationDataPacket {
    Tcp(Vec<u8>),
    Udp {
        destination_address: UnifiedAddress,
        data: Vec<u8>,
    },
}
pub enum DestinationTransport {
    Tcp(Framed<TcpStream, DestinationDataTcpCodec>),
    Udp {
        destination_address: UnifiedAddress,
        destination_udp_socket: UdpSocket,
    },
}
impl DestinationTransport {
    pub async fn new_tcp(
        dst_addresses: &UnifiedAddress,
        server_state: ServerState,
    ) -> Result<Self, ProxyError> {
        let dst_addresses: Vec<SocketAddr> = dst_addresses.try_into()?;
        let dest_socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
        let random_dst_addr_index = random::<usize>() % dst_addresses.len();
        dest_socket.connect_timeout(
            &dst_addresses[random_dst_addr_index].into(),
            Duration::from_secs(*server_state.config().dst_connect_timeout()),
        )?;
        dest_socket.set_nonblocking(true)?;
        dest_socket.set_reuse_address(true)?;
        dest_socket.set_keepalive(true)?;
        let keepalive = TcpKeepalive::new().with_time(Duration::from_secs(
            *server_state.config().dst_tcp_keepalive_time(),
        )).with_interval(Duration::from_secs(*server_state.config().dst_tcp_keepalive_interval()));
        #[cfg(target_os = "linux")]
        let keepalive = keepalive.with_retries(*server_state.config().dst_tcp_keepalive_retry());
        dest_socket.set_tcp_keepalive(&keepalive)?;
        dest_socket.set_nonblocking(true)?;
        dest_socket.set_nodelay(true)?;
        dest_socket.set_read_timeout(Some(Duration::from_secs(
            *server_state.config().dst_read_timeout(),
        )))?;
        dest_socket.set_write_timeout(Some(Duration::from_secs(
            *server_state.config().dst_write_timeout(),
        )))?;
        let dst_tcp_stream = TcpStream::connect(dst_addresses.as_slice()).await?;
        Ok(DestinationTransport::Tcp(Framed::with_capacity(
            dst_tcp_stream,
            DestinationDataTcpCodec::new(),
            *server_state.config().dst_buffer_size(),
        )))
    }
    pub async fn new_udp(
        dst_addresses: &UnifiedAddress,
        _server_state: ServerState,
    ) -> Result<Self, ProxyError> {
        let dst_addresses_clone = dst_addresses.clone();
        let dst_addresses: Vec<SocketAddr> = dst_addresses.try_into()?;
        let dst_udp_socket = UdpSocket::bind("0.0.0.0:0").await?;
        dst_udp_socket.connect(dst_addresses.as_slice()).await?;
        Ok(DestinationTransport::Udp {
            destination_address: dst_addresses_clone,
            destination_udp_socket: dst_udp_socket,
        })
    }
    pub fn split(self) -> (DestinationTransportWrite, DestinationTransportRead) {
        match self {
            DestinationTransport::Tcp(framed) => {
                let (framed_write, framed_read) = framed.split::<BytesMut>();
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
