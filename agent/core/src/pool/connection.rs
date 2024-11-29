use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
#[derive(Debug)]
pub struct PooledProxyConnection {
    tcp_stream: TcpStream,
}
impl PooledProxyConnection {
    pub fn new(tcp_stream: TcpStream) -> PooledProxyConnection {
        PooledProxyConnection {
            tcp_stream,
        }
    }
}
impl AsyncRead for PooledProxyConnection {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let tcp_stream = Pin::new(&mut self.get_mut().tcp_stream);
        tcp_stream.poll_read(cx, buf)
    }
}
impl AsyncWrite for PooledProxyConnection {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        let tcp_stream = Pin::new(&mut self.get_mut().tcp_stream);
        tcp_stream.poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let tcp_stream = Pin::new(&mut self.get_mut().tcp_stream);
        tcp_stream.poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let tcp_stream = Pin::new(&mut self.get_mut().tcp_stream);
        tcp_stream.poll_shutdown(cx)
    }
}