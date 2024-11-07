use crate::bo::session::SessionBuilderError;
use crate::bo::state::ServerStateBuilderError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use base64::DecodeError;
use hex::FromHexError;
use ppaass_crypto::error::CryptoError;
use ppaass_domain::error::DomainError;
use ppaass_domain::session::{CreateSessionResponseBuilderError, GetSessionResponseBuilderError};
use thiserror::Error;
#[derive(Debug, Error)]
pub enum ServerError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Agent tcp connection exhausted")]
    AgentTcpConnectionExhausted,
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error("Rsa crypto not exist: {0}")]
    RsaCryptoNotExist(String),
    #[error("Require encryption for session: {0}.")]
    SessionRequireEncryptionKey(String),
    #[error("Require auth token for session.")]
    SessionRequireAuthToken,
    #[error("Session [{0}] not exist.")]
    SessionNotExist(String),
    #[error("Destination transport not exist.")]
    DestinationTransportNotExist,
    #[error("Fail to lock session repository.")]
    SessionRepositoryLock,
    #[error(transparent)]
    SessionBuilder(#[from] SessionBuilderError),
    #[error(transparent)]
    CreateSessionResponseBuilder(#[from] CreateSessionResponseBuilderError),
    #[error(transparent)]
    GetSessionResponseBuilder(#[from] GetSessionResponseBuilderError),
    #[error(transparent)]
    ServerStateBuilder(#[from] ServerStateBuilderError),
    #[error(transparent)]
    FromHex(#[from] FromHexError),
    #[error(transparent)]
    Base64Decode(#[from] DecodeError),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        match self {
            Self::SessionNotExist(session_token) => {
                (StatusCode::NOT_FOUND, session_token).into_response()
            }
            Self::RsaCryptoNotExist(auth_token) => {
                (StatusCode::NOT_FOUND, auth_token).into_response()
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
