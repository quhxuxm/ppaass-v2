use crate::error::ProxyError;
use bytes::BytesMut;
use futures::Stream;
use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::ReadBuf;
use tokio::net::{TcpStream, UdpSocket};
use tokio_io_timeout::TimeoutStream;
use tokio_util::codec::{BytesCodec, Framed};
const UDP_READ_BUFFER_SIZE: usize = 65536;
pub enum DestinationTransportRead {
    Tcp(SplitStream<Framed<TimeoutStream<TcpStream>, BytesCodec>>),
    Udp(Arc<UdpSocket>),
}
impl Stream for DestinationTransportRead {
    type Item = Result<BytesMut, ProxyError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            DestinationTransportRead::Tcp(inner_tcp_stream) => {
                inner_tcp_stream.poll_next_unpin(cx).map_err(ProxyError::Io)
            }
            DestinationTransportRead::Udp(inner_udp_socket) => {
                let mut read_buf = [0u8; UDP_READ_BUFFER_SIZE];
                let mut read_buf = ReadBuf::new(&mut read_buf);
                match inner_udp_socket.poll_recv(cx, &mut read_buf) {
                    Poll::Ready(Ok(())) => Poll::Ready(Some(Ok(read_buf.filled().into()))),
                    Poll::Ready(Err(e)) => Poll::Ready(Some(Err(ProxyError::Io(e)))),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}
