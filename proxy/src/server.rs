use crate::codec::AgentConnectionCodec;
use crate::config::Configuration;
use crate::error::ServerError;
use futures_util::{SinkExt, StreamExt};
use ppaass_v2_domain::message::PpaassMessagePacket;
use std::collections::VecDeque;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc::channel;
use tokio_util::codec::Framed;
use tracing::error;
pub struct Server {
    configuration: Arc<Configuration>,
}

impl Server {
    pub fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub async fn run(&self) -> Result<(), ServerError> {
        let tcp_listener = TcpListener::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            self.configuration.port(),
        ))
        .await?;
        loop {
            let (agent_tcp_stream, agent_socket_address) = match tcp_listener.accept().await {
                Ok(agent_connection) => agent_connection,
                Err(e) => {
                    error!(
                        error = ?e,
                        "Fail to accept agent tcp connection."
                    );
                    continue;
                }
            };
            let agent_framed = Framed::new(agent_tcp_stream, AgentConnectionCodec::new());
            let (mut agent_write, agent_read) = agent_framed.split();

            let (agent_write_tx, mut agent_write_rx) = channel::<PpaassMessagePacket>(1024);
            tokio::spawn(async move {
                while let Some(msg) = agent_write_rx.recv().await {
                    if let Err(e) = agent_write.send(msg).await {
                        error!(error=?e, "Fail to write data to agent.");
                        return;
                    };
                }
            });
        }
    }
}
