pub mod read;
pub mod write;
mod codec;
use crate::bo::state::ServerState;
use crate::destination::codec::DestinationDataTcpCodec;
use crate::destination::read::DestinationTransportRead;
use crate::destination::write::DestinationTransportWrite;
use crate::error::ProxyError;
use bytes::BytesMut;
use futures_util::StreamExt;
use ppaass_domain::address::UnifiedAddress;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpStream, UdpSocket};
use tokio_io_timeout::TimeoutStream;
use tokio_util::codec::Framed;
pub enum DestinationDataPacket {
    Tcp(Vec<u8>),
    Udp {
        destination_address: UnifiedAddress,
        data: Vec<u8>,
    },
}
pub enum DestinationTransport {
    Tcp(Framed<TimeoutStream<TcpStream>, DestinationDataTcpCodec>),
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
        let dst_tcp_stream = TcpStream::connect(dst_addresses.as_slice()).await?;
        let mut dst_tcp_stream = TimeoutStream::new(dst_tcp_stream);
        dst_tcp_stream.set_read_timeout(Some(Duration::from_secs(
            *server_state.config().dst_read_timeout(),
        )));
        dst_tcp_stream.set_write_timeout(Some(Duration::from_secs(
            *server_state.config().dst_write_timeout(),
        )));
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
                destination_udp_socket
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
