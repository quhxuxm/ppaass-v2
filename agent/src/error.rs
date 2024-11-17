use ppaass_crypto::error::CryptoError;
use ppaass_domain::error::DomainError;
use ppaass_domain::relay::{RelayInfoBuilderError, RelayUpgradeFailureReason};
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
    HttpClientWebSocket(#[from] reqwest_websocket::Error),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error("Rsa crypto not exist: {0}")]
    RsaCryptoNotExist(String),
    #[error(transparent)]
    RelayInfoBuilder(#[from] RelayInfoBuilderError),
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    ByteCodec(#[from] bytecodec::Error),
    #[error(transparent)]
    ParseUrl(#[from] url::ParseError),
    #[error("Unknown host from target url")]
    UnknownHostFromTargetUrl(String),
    #[error("Fail to upgrade relay websocket: {0} ")]
    RelayWebSocketUpgrade(RelayUpgradeFailureReason),
    #[error("Fail to lock agent session")]
    AgentSessionLock,
}
