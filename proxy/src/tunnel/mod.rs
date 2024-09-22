use crate::codec::AgentConnectionCodec;
use crate::error::ServerError;
use crate::tunnel::tcp::TcpTunnel;
use crate::tunnel::udp::UdpTunnel;
use futures_util::stream::SplitStream;
use futures_util::{Sink, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
mod tcp;
mod udp;
pub enum Tunnel {
    Tcp(TcpTunnel),
    Udp(UdpTunnel),
}

impl Tunnel {
    pub async fn detect(
        mut agent_read: SplitStream<Framed<TcpStream, AgentConnectionCodec>>,
    ) -> Result<Self, ServerError> {
        let handshake = agent_read.next().await.ok_or(ServerError::Other(
            "Nothing read from agent side".to_string(),
        ))??;

        Tunnel::Tcp(TcpTunnel::new())
    }
}
