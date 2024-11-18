use crate::bo::event::ProxyServerEvent;
use crate::bo::session::SessionBuilder;
use crate::bo::state::ServerState;
use crate::error::ProxyError;
use crate::publish_server_event;
use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use ppaass_crypto::random_32_bytes;
use ppaass_crypto::rsa::RsaCryptoFetcher;
use ppaass_domain::generate_uuid;
use ppaass_domain::session::{
    CreateSessionRequest, CreateSessionResponse, Encryption, GetSessionResponse,
};
use std::sync::Arc;
pub async fn get_session(
    State(server_state): State<Arc<ServerState>>,
    Path(session_token): Path<String>,
) -> Result<Json<GetSessionResponse>, ProxyError> {
    let session_repository = server_state
        .session_repository()
        .lock()
        .map_err(|_| ProxyError::SessionRepositoryLock)?;
    let session = session_repository
        .get(&session_token)
        .ok_or(ProxyError::SessionNotExist(session_token.clone()))?;
    Ok(Json(GetSessionResponse {
        session_token,
        auth_token: session.auth_token().to_owned(),
        relay_infos: session.relays().to_owned(),
    }))
}
pub async fn get_all_sessions(
    State(server_state): State<Arc<ServerState>>,
) -> Result<Json<Vec<GetSessionResponse>>, ProxyError> {
    let session_repository = server_state
        .session_repository()
        .lock()
        .map_err(|_| ProxyError::SessionRepositoryLock)?;
    let result = session_repository
        .iter()
        .filter_map(|(k, v)| {
            Some(GetSessionResponse {
                session_token: k.to_owned(),
                auth_token: v.auth_token().to_owned(),
                relay_infos: v.relays().to_owned(),
            })
        })
        .collect::<Vec<GetSessionResponse>>();
    Ok(Json(result))
}
// #[axum::debug_handler]
pub async fn create_session(
    State(server_state): State<Arc<ServerState>>,
    Json(CreateSessionRequest {
             agent_encryption,
             auth_token,
         }): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, ProxyError> {
    let rsa_crypto_fetcher = server_state.rsa_crypto_fetcher();
    let rsa_crypto = rsa_crypto_fetcher
        .fetch(&auth_token)?
        .ok_or(ProxyError::RsaCryptoNotExist(auth_token.clone()))?;
    let session_token = generate_uuid();
    let random_raw_proxy_aes_token = random_32_bytes();
    let agent_encryption = match &agent_encryption {
        Encryption::Plain => Encryption::Plain,
        Encryption::Aes(rsa_encrypted_aes_token) => {
            Encryption::Aes(rsa_crypto.decrypt(rsa_encrypted_aes_token)?.into())
        }
    };
    let proxy_encryption = Encryption::Aes(random_raw_proxy_aes_token.clone());
    let session_creation_time = Utc::now();
    let mut session_builder = SessionBuilder::default();
    session_builder
        .session_token(session_token.clone())
        .agent_encryption(agent_encryption)
        .auth_token(auth_token)
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
    let proxy_encryption = Encryption::Aes(rsa_crypto.encrypt(&random_raw_proxy_aes_token)?.into());
    Ok(Json(CreateSessionResponse {
        proxy_encryption,
        session_token,
    }))
}
