use crate::destination::codec::DestinationDataTcpCodec;
use crate::destination::DestinationDataPacket;
use crate::error::ProxyError;
use futures::Stream;
use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use ppaass_domain::address::UnifiedAddress;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::ReadBuf;
use tokio::net::{TcpStream, UdpSocket};
use tokio_io_timeout::TimeoutStream;
use tokio_util::codec::Framed;
const UDP_READ_BUFFER_SIZE: usize = 65536;
pub enum DestinationTransportRead {
    Tcp(SplitStream<Framed<TimeoutStream<TcpStream>, DestinationDataTcpCodec>>),
    Udp {
        destination_address: UnifiedAddress,
        destination_udp_socket: Arc<UdpSocket>,
    },
}
impl Stream for DestinationTransportRead {
    type Item = Result<DestinationDataPacket, ProxyError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            DestinationTransportRead::Tcp(inner_tcp_stream) => inner_tcp_stream.poll_next_unpin(cx),
            DestinationTransportRead::Udp {
                destination_address,
                destination_udp_socket,
            } => {
                let mut read_buf = [0u8; UDP_READ_BUFFER_SIZE];
                let mut read_buf = ReadBuf::new(&mut read_buf);
                match destination_udp_socket.poll_recv(cx, &mut read_buf) {
                    Poll::Ready(Ok(())) => Poll::Ready(Some(Ok(DestinationDataPacket::Udp {
                        data: read_buf.filled().into(),
                        destination_address: destination_address.clone(),
                    }))),
                    Poll::Ready(Err(e)) => Poll::Ready(Some(Err(ProxyError::Io(e)))),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}
