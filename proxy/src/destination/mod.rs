pub mod read;
pub mod write;
use crate::destination::read::DestinationTransportRead;
use crate::destination::write::DestinationTransportWrite;
use bytes::BytesMut;
use futures_util::StreamExt;
use tokio::net::{TcpStream, UdpSocket};
use tokio_util::codec::{BytesCodec, Framed};
pub enum DestinationTransport {
    Tcp(Framed<TcpStream, BytesCodec>),
    Udp(UdpSocket),
}

impl DestinationTransport {
    pub fn new_tcp(dst_tcp_stream: TcpStream) -> Self {
        DestinationTransport::Tcp(Framed::new(dst_tcp_stream, BytesCodec::new()))
    }

    pub fn new_udp(dst_socket: UdpSocket) -> Self {
        DestinationTransport::Udp(dst_socket)
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
            DestinationTransport::Udp(_) => {
                todo!()
            }
        }
    }
}
