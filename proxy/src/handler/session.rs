use crate::bo::session::SessionBuilder;
use crate::bo::state::ServerState;
use crate::error::ServerError;
use axum::extract::{Path, State};
use axum::Json;
use ppaass_crypto::random_32_bytes;
use ppaass_crypto::rsa::RsaCryptoFetcher;
use ppaass_domain::session::{
    CreateSessionRequest, CreateSessionResponse, CreateSessionResponseBuilder, Encryption,
    GetSessionResponse, GetSessionResponseBuilder,
};
use std::sync::Arc;
use uuid::Uuid;
pub async fn get_session(
    State(server_state): State<Arc<ServerState>>,
    Path(session_token): Path<String>,
) -> Result<Json<GetSessionResponse>, ServerError> {
    let session_repository = server_state
        .session_repository()
        .lock()
        .map_err(|_| ServerError::SessionRepositoryLock)?;
    let session = session_repository
        .get(&session_token)
        .ok_or(ServerError::SessionNotExist(session_token.clone()))?;
    let mut get_session_response_builder = GetSessionResponseBuilder::default();
    get_session_response_builder.session_token(session_token);
    get_session_response_builder.auth_token(session.auth_token().to_owned());
    let get_session_response = get_session_response_builder.build()?;
    Ok(Json(get_session_response))
}

pub async fn get_all_sessions(
    State(server_state): State<Arc<ServerState>>,
) -> Result<Json<Vec<GetSessionResponse>>, ServerError> {
    let session_repository = server_state
        .session_repository()
        .lock()
        .map_err(|_| ServerError::SessionRepositoryLock)?;
    let result = session_repository
        .iter()
        .filter_map(|(k, v)| {
            let mut get_session_response_builder = GetSessionResponseBuilder::default();
            get_session_response_builder.session_token(k.to_owned());
            get_session_response_builder.auth_token(v.auth_token().to_owned());
            let get_session_response = match get_session_response_builder.build() {
                Ok(response) => response,
                Err(_) => {
                    return None;
                }
            };
            Some(get_session_response)
        })
        .collect::<Vec<GetSessionResponse>>();
    Ok(Json(result))
}

pub async fn create_session(
    State(server_state): State<Arc<ServerState>>,
    Json(create_session_request): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, ServerError> {
    let rsa_crypto_fetcher = server_state.rsa_crypto_fetcher();
    let rsa_crypto = rsa_crypto_fetcher
        .fetch(create_session_request.auth_token())?
        .ok_or(ServerError::RsaCryptoNotExist(
            create_session_request.auth_token().to_owned(),
        ))?;
    let session_token = Uuid::new_v4().to_string();
    let random_raw_proxy_aes_token = random_32_bytes();
    let proxy_encryption = Encryption::Aes(rsa_crypto.encrypt(&random_raw_proxy_aes_token)?.into());
    let agent_encryption = match create_session_request.agent_encryption() {
        Encryption::Plain => Encryption::Plain,
        Encryption::Aes(rsa_encrypted_aes_token) => {
            Encryption::Aes(rsa_crypto.decrypt(rsa_encrypted_aes_token)?.into())
        }
    };
    let mut session_builder = SessionBuilder::default();
    session_builder
        .session_token(session_token.clone())
        .agent_encryption(agent_encryption)
        .auth_token(create_session_request.auth_token().to_owned())
        .proxy_encryption(proxy_encryption.clone());
    let session = session_builder.build()?;
    let mut session_repository = server_state
        .session_repository()
        .lock()
        .map_err(|_| ServerError::SessionRepositoryLock)?;
    session_repository.insert(session.session_token().to_owned(), session);
    let mut create_session_response_builder = CreateSessionResponseBuilder::default();
    create_session_response_builder
        .proxy_encryption(proxy_encryption)
        .session_token(session_token);
    let create_session_response = create_session_response_builder.build()?;
    Ok(Json(create_session_response))
}
