use crate::bo::state::ServerStateBuilderError;
use ppaass_codec::error::CodecError;
use ppaass_crypto::error::CryptoError;
use ppaass_domain::error::DomainError;
use std::net::AddrParseError;
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
    Crypto(#[from] CryptoError),
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error("Rsa crypto not exist: {0}")]
    RsaCryptoNotExist(String),
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    ByteCodec(#[from] bytecodec::Error),
    #[error(transparent)]
    ParseUrl(#[from] url::ParseError),
    #[error("Unknown host from target url")]
    UnknownHostFromTargetUrl(String),
    #[error(transparent)]
    ServerStateBuilder(#[from] ServerStateBuilderError),
    #[error("Proxy connection pool error: {0}")]
    ProxyConnectionPool(String),
    #[error("Proxy connection exhausted")]
    ProxyConnectionExhausted,
    #[error("Invalid proxy data type")]
    InvalidProxyDataType,
    #[error(transparent)]
    AddrParse(#[from] AddrParseError),
    #[error("Unknown error happen: {0}")]
    Unknown(String),
    #[error(transparent)]
    ConnectProxyTimeout(#[from] tokio::time::error::Elapsed),
}
impl From<AgentError> for std::io::Error {
    fn from(value: AgentError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, value)
    }
}
