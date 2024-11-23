use crate::error::ProxyError;
use bytes::BytesMut;
use futures::Sink;
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::net::{TcpStream, UdpSocket};
use tokio_io_timeout::TimeoutStream;
use tokio_util::codec::{BytesCodec, Framed};
pub enum DestinationTransportWrite {
    Tcp(SplitSink<Framed<TimeoutStream<TcpStream>, BytesCodec>, BytesMut>),
    Udp(Arc<UdpSocket>),
}
impl Sink<BytesMut> for DestinationTransportWrite {
    type Error = ProxyError;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            DestinationTransportWrite::Tcp(inner_sink) => {
                inner_sink.poll_ready_unpin(cx).map_err(ProxyError::Io)
            }
            DestinationTransportWrite::Udp(udp_socket) => {
                udp_socket.poll_send_ready(cx).map_err(ProxyError::Io)
            }
        }
    }
    fn start_send(self: Pin<&mut Self>, item: BytesMut) -> Result<(), Self::Error> {
        match self.get_mut() {
            DestinationTransportWrite::Tcp(inner_sink) => {
                inner_sink.start_send_unpin(item).map_err(ProxyError::Io)
            }
            DestinationTransportWrite::Udp(_) => Ok(()),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            DestinationTransportWrite::Tcp(inner_sink) => {
                inner_sink.poll_flush_unpin(cx).map_err(ProxyError::Io)
            }
            DestinationTransportWrite::Udp(_) => Poll::Ready(Ok(())),
        }
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            DestinationTransportWrite::Tcp(inner_sink) => {
                inner_sink.poll_close_unpin(cx).map_err(ProxyError::Io)
            }
            DestinationTransportWrite::Udp(_) => Poll::Ready(Ok(())),
        }
    }
}
