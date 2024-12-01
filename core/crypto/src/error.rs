use thiserror::Error;
#[derive(Error, Debug)]

pub enum CryptoError {
    #[error("Crypto error happen because of io: {_0:?}")]
    Io(#[from] std::io::Error),
    #[error("Aes crypto error: {_0}")]
    Aes(String),
    #[error("Rsa crypto error: {_0}")]
    Rsa(String),
}
