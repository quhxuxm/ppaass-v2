use bytes::BytesMut;
use futures::Sink;
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::TcpStream;
use tokio_util::codec::{BytesCodec, Framed};
pub enum DestinationTransportWrite {
    Tcp(SplitSink<Framed<TcpStream, BytesCodec>, BytesMut>),
    Udp(),
}

impl Sink<BytesMut> for DestinationTransportWrite {
    type Error = std::io::Error;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            DestinationTransportWrite::Tcp(inner_sink) => inner_sink.poll_ready_unpin(cx),
            DestinationTransportWrite::Udp() => {
                todo!()
            }
        }
    }
    fn start_send(self: Pin<&mut Self>, item: BytesMut) -> Result<(), Self::Error> {
        match self.get_mut() {
            DestinationTransportWrite::Tcp(inner_sink) => inner_sink.start_send_unpin(item),
            DestinationTransportWrite::Udp() => {
                todo!()
            }
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            DestinationTransportWrite::Tcp(inner_sink) => inner_sink.poll_flush_unpin(cx),
            DestinationTransportWrite::Udp() => {
                todo!()
            }
        }
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            DestinationTransportWrite::Tcp(inner_sink) => inner_sink.poll_close_unpin(cx),
            DestinationTransportWrite::Udp() => {
                todo!()
            }
        }
    }
}
