use crate::bo::session::SessionBuilderError;
use crate::bo::state::ServerStateBuilderError;
use ppaass_codec::error::CodecError;
use ppaass_crypto::error::CryptoError;
use ppaass_domain::error::DomainError;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum ProxyError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Agent tcp connection exhausted")]
    AgentTcpConnectionExhausted,
    #[error("Agent tcp connection fail to reunite: {0}")]
    AgentTcpConnectionReunite(String),
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
    ServerStateBuilder(#[from] ServerStateBuilderError),
    #[error(transparent)]
    FromHex(#[from] CodecError),

}
