use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
#[derive(Debug)]
pub struct PooledProxyConnection<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    inner: T,
}
impl<T> PooledProxyConnection<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(inner: T) -> PooledProxyConnection<T> {
        PooledProxyConnection {
            inner,
        }
    }
}
impl<T> AsyncRead for PooledProxyConnection<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let inner = Pin::new(&mut self.get_mut().inner);
        inner.poll_read(cx, buf)
    }
}
impl<T> AsyncWrite for PooledProxyConnection<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        let inner = Pin::new(&mut self.get_mut().inner);
        inner.poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let inner = Pin::new(&mut self.get_mut().inner);
        inner.poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let inner = Pin::new(&mut self.get_mut().inner);
        inner.poll_shutdown(cx)
    }
}