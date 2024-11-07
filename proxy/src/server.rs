use crate::bo::config::Config;
use crate::bo::event::ServerEvent;
use crate::bo::state::{ServerState, ServerStateBuilder};
use crate::crypto::ProxyRsaCryptoFetcher;
use crate::error::ServerError;
use crate::{handler, publish_server_event};
use axum::routing::{get, post};
use axum::Router;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{channel, Receiver};
use tracing::error;
pub struct Server {
    state: Arc<ServerState>,
}

impl Server {
    pub fn new(config: Arc<Config>) -> Result<Self, ServerError> {
        let mut server_state_builder = ServerStateBuilder::default();
        server_state_builder
            .config(config.clone())
            .rsa_crypto_fetcher(Arc::new(ProxyRsaCryptoFetcher::new(config)?))
            .session_repository(Arc::new(Mutex::new(HashMap::new())));
        Ok(Self {
            state: Arc::new(server_state_builder.build()?),
        })
    }

    async fn concrete_start_server(state: Arc<ServerState>) -> Result<(), ServerError> {
        let session_server_port = *state.config().port();
        let app = Router::new()
            .route("/session/create", post(handler::create_session))
            .route("/session", get(handler::get_all_sessions))
            .route("/session/:session_token", get(handler::get_session))
            .route(
                "/relay/:session_token/:relay_info",
                get(handler::prepare_dest_transport),
            )
            .with_state(state);
        let session_server_listener = TcpListener::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            session_server_port,
        ))
        .await?;
        axum::serve(session_server_listener, app).await?;
        Ok(())
    }

    pub async fn start(&self) -> Result<Receiver<ServerEvent>, ServerError> {
        let (server_event_tx, server_event_rx) = channel::<ServerEvent>(1024);
        {
            let server_state = self.state.clone();

            let server_event_tx = server_event_tx.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::concrete_start_server(server_state).await {
                    publish_server_event(server_event_tx, ServerEvent::ServerStartFail).await;
                    error!("Fail to start server: {e:?}")
                }
            });
        }
        publish_server_event(server_event_tx, ServerEvent::ServerStartup).await;
        Ok(server_event_rx)
    }
}
