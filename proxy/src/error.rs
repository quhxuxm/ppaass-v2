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
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error("Rsa crypto not exist: {0}")]
    RsaCryptoNotExist(String),
    #[error(transparent)]
    ServerStateBuilder(#[from] ServerStateBuilderError),
    #[error(transparent)]
    FromHex(#[from] CodecError),
}
impl From<ProxyError> for std::io::Error {
    fn from(value: ProxyError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, value)
    }
}
