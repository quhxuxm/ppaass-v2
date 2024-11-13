pub mod read;
pub mod write;
use crate::destination::read::DestinationTransportRead;
use crate::destination::write::DestinationTransportWrite;
use crate::error::ProxyError;
use bytes::BytesMut;
use futures_util::StreamExt;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpStream, UdpSocket};
use tokio_util::codec::{BytesCodec, Framed};
pub enum DestinationTransport {
    Tcp(Framed<TcpStream, BytesCodec>),
    Udp(UdpSocket),
}
impl DestinationTransport {
    pub async fn new_tcp(dst_addresses: Vec<SocketAddr>) -> Result<Self, ProxyError> {
        let dst_tcp_stream = TcpStream::connect(dst_addresses.as_slice()).await?;
        Ok(DestinationTransport::Tcp(Framed::with_capacity(dst_tcp_stream, BytesCodec::new(), 1024 * 1024 * 64)))
    }
    pub async fn new_udp(dst_addresses: Vec<SocketAddr>) -> Result<Self, ProxyError> {
        let dst_udp_socket = UdpSocket::bind("0.0.0.0:0").await?;
        dst_udp_socket.connect(dst_addresses.as_slice()).await?;
        Ok(DestinationTransport::Udp(dst_udp_socket))
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
            DestinationTransport::Udp(udp_socket) => {
                let udp_socket = Arc::new(udp_socket);
                (DestinationTransportWrite::Udp(udp_socket.clone()), DestinationTransportRead::Udp(udp_socket))
            }
        }
    }
}
