use crate::bo::config::Config;
use crate::bo::event::ProxyServerEvent;
use crate::bo::state::{ServerState, ServerStateBuilder};
use crate::codec::SessionInitCodec;
use crate::crypto::ProxyRsaCryptoFetcher;
use crate::error::ProxyError;
use crate::handler::create_session;
use crate::{handler, publish_server_event};
use chrono::Utc;
use futures_util::StreamExt;
use ppaass_codec::SessionInitRequestDecoder;
use ppaass_domain::session::SessionInitRequest;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{channel, Receiver};
use tokio::time::sleep;
use tracing::{debug, error};
pub struct ProxyServer {
    state: Arc<ServerState>,
}
impl ProxyServer {
    pub fn new(config: Arc<Config>) -> Result<(Self, Receiver<ProxyServerEvent>), ProxyError> {
        let (server_event_tx, server_event_rx) = channel::<ProxyServerEvent>(1024);
        let mut server_state_builder = ServerStateBuilder::default();
        server_state_builder
            .config(config.clone())
            .rsa_crypto_fetcher(Arc::new(ProxyRsaCryptoFetcher::new(config)?))
            .session_repository(Arc::new(Mutex::new(HashMap::new())))
            .server_event_tx(Arc::new(server_event_tx));
        Ok((
            Self {
                state: Arc::new(server_state_builder.build()?),
            },
            server_event_rx,
        ))
    }
    async fn concrete_start_server(state: Arc<ServerState>) -> Result<(), ProxyError> {
        let server_port = *state.config().port();
        let server_listener = TcpListener::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            server_port,
        ))
            .await?;
        loop {
            let (agent_tcp_stream, agent_socket_addr) = server_listener.accept().await?;
            debug!("Accept agent tcp connection from: {agent_socket_addr}");
            let (auth_token, agent_tcp_stream) = create_session(agent_tcp_stream, state.clone()).await?;
        }
        Ok(())
    }
    pub async fn start(
        &self,
        server_event_rx: Receiver<ProxyServerEvent>,
    ) -> Result<Receiver<ProxyServerEvent>, ProxyError> {
        {
            let server_state = self.state.clone();
            let server_event_tx_clone = server_state.server_event_tx().clone();
            tokio::spawn(async move {
                if let Err(e) = Self::concrete_start_server(server_state).await {
                    publish_server_event(&server_event_tx_clone, ProxyServerEvent::ServerStartFail)
                        .await;
                    error!("Fail to start server: {e:?}")
                }
            });
            let server_state = self.state.clone();
            let server_event_tx_clone = server_state.server_event_tx().clone();
            tokio::spawn(async move {
                loop {
                    let session_token_to_remove = {
                        let mut lock = match server_state.session_repository().lock() {
                            Ok(lock) => lock,
                            Err(e) => {
                                error!("Fail to lock session repository: {e:?}");
                                return;
                            }
                        };
                        let mut session_token_to_remove = vec![];
                        lock.iter().for_each(|(session_token, session)| {
                            let pass = Utc::now() - session.update_time();
                            if pass.num_minutes() > 15 {
                                session_token_to_remove.push(session_token.clone());
                            }
                        });
                        for session_token in session_token_to_remove.iter() {
                            lock.remove(session_token);
                            debug!("Remove session from repository: {session_token}");
                        }
                        session_token_to_remove
                    };
                    for session_token in session_token_to_remove.iter() {
                        publish_server_event(
                            &server_event_tx_clone,
                            ProxyServerEvent::SessionClosed(session_token.clone()),
                        )
                            .await;
                    }
                    sleep(Duration::from_secs(60 * 5)).await;
                }
            });
        }
        publish_server_event(
            self.state.server_event_tx(),
            ProxyServerEvent::ServerStartup,
        )
            .await;
        Ok(server_event_rx)
    }
}
