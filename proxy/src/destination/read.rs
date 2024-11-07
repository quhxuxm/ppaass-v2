use bytes::BytesMut;
use futures::Stream;
use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::{TcpStream, UdpSocket};
use tokio_util::codec::{BytesCodec, Framed};
pub enum DestinationTransportRead {
    Tcp(SplitStream<Framed<TcpStream, BytesCodec>>),
    Udp(UdpSocket),
}

impl Stream for DestinationTransportRead {
    type Item = Result<BytesMut, std::io::Error>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            DestinationTransportRead::Tcp(inner_tcp_stream) => inner_tcp_stream.poll_next_unpin(cx),
            DestinationTransportRead::Udp(inner_udp_socket) => {
                todo!()
            }
        }
    }
}
