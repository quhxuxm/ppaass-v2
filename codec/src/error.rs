use ppaass_domain::error::DomainError;
use ppaass_domain::packet::PpaassPacketBuilderError;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum CodecError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Crypto(#[from] ppaass_crypto::error::CryptoError),
    #[error(transparent)]
    DomainBuilder(#[from] PpaassPacketBuilderError),
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("{0}")]
    Other(String),
}
