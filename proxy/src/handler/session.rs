use crate::bo::event::ProxyServerEvent;
use crate::bo::session::SessionBuilder;
use crate::bo::state::ServerState;
use crate::codec::SessionInitCodec;
use crate::error::ProxyError;
use crate::publish_server_event;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use ppaass_crypto::random_32_bytes;
use ppaass_domain::generate_uuid;
use ppaass_domain::session::{Encryption, SessionInitRequest, SessionInitResponse, SessionInitResponseStatus};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
/// Create session in proxy side and return the original agent tcp connection
pub async fn create_session(
    agent_tcp_stream: TcpStream,
    server_state: Arc<ServerState>,
) -> Result<(String, TcpStream), ProxyError> {
    let agent_session_init_framed = Framed::with_capacity(agent_tcp_stream, SessionInitCodec::new(server_state.rsa_crypto_fetcher().clone()), *server_state.config().agent_buffer_size());
    let (mut agent_session_init_tx, mut agent_session_init_rx) = agent_session_init_framed.split();
    let SessionInitRequest {
        agent_encryption, auth_token
    } = agent_session_init_rx.next().await.ok_or(ProxyError::AgentTcpConnectionExhausted)??;
    let session_token = generate_uuid();
    let proxy_encryption = Encryption::Aes(random_32_bytes());
    let session_creation_time = Utc::now();
    let mut session_builder = SessionBuilder::default();
    session_builder
        .session_token(session_token.clone())
        .agent_encryption(agent_encryption)
        .auth_token(auth_token.clone())
        .proxy_encryption(proxy_encryption.clone())
        .create_time(session_creation_time)
        .update_time(session_creation_time)
        .relays(vec![]);
    let session = session_builder.build()?;
    {
        let mut session_repository = server_state
            .session_repository()
            .lock()
            .map_err(|_| ProxyError::SessionRepositoryLock)?;
        session_repository.insert(session.session_token().to_owned(), session);
    }
    publish_server_event(
        server_state.server_event_tx(),
        ProxyServerEvent::SessionStarted(session_token.clone()),
    )
        .await;
    let session_init_response = SessionInitResponse {
        proxy_encryption,
        session_token,
        status: SessionInitResponseStatus::Success,
    };
    agent_session_init_tx.send((auth_token.clone(), session_init_response)).await?;
    let agent_tcp_stream = agent_session_init_rx.reunite(agent_session_init_tx).map_err(|e| ProxyError::AgentTcpConnectionReunite(e.to_string()))?;
    Ok((auth_token, agent_tcp_stream.into_inner()))
}
