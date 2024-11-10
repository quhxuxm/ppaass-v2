use ppaass_crypto::error::CryptoError;
use ppaass_domain::error::DomainError;
use ppaass_domain::relay::RelayInfoBuilderError;
use ppaass_domain::session::CreateSessionRequestBuilderError;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum AgentError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Client tcp connection exhausted")]
    ClientTcpConnectionExhausted,
    #[error("Unsupported protocol: socks4")]
    UnsupportedSocksV4Protocol,
    #[error(transparent)]
    HttpClient(#[from] reqwest::Error),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error("Rsa crypto not exist: {0}")]
    RsaCryptoNotExist(String),
    #[error(transparent)]
    CreateSessionRequestBuilder(#[from] CreateSessionRequestBuilderError),
    #[error(transparent)]
    RelayInfoBuilder(#[from] RelayInfoBuilderError),
    #[error(transparent)]
    Domain(#[from] DomainError),
}
