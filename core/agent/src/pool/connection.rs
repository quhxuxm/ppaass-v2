use crate::config::Config;
use chrono::{DateTime, Utc};
use std::io::Error;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
/// Pooled proxy connection
/// T is the underline async read and write
#[derive(Debug)]
pub struct PooledProxyConnection<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    inner: T,
    config: Arc<Config>,
    last_check_time: DateTime<Utc>,
    create_time: DateTime<Utc>,
}
impl<T> PooledProxyConnection<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(inner: T, config: Arc<Config>) -> PooledProxyConnection<T> {
        PooledProxyConnection {
            inner,
            config,
            last_check_time: Utc::now(),
            create_time: Utc::now(),
        }
    }
    pub fn need_check(&self) -> bool {
        let now = Utc::now();
        let delta = now - self.last_check_time;
        delta.num_seconds() > *self.config.proxy_connection_check_interval()
    }

    pub fn need_close(&self) -> bool {
        let now = Utc::now();
        let delta = now - self.create_time;
        delta.num_seconds() > *self.config.proxy_connection_max_lifetime()
    }

    pub fn update_check_time(&mut self) {
        self.last_check_time = Utc::now();
    }
    pub fn last_check_time(&self) -> &DateTime<Utc> {
        &self.last_check_time
    }
}
impl<T> AsyncRead for PooledProxyConnection<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let inner = Pin::new(&mut self.get_mut().inner);
        inner.poll_read(cx, buf)
    }
}
impl<T> AsyncWrite for PooledProxyConnection<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
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
