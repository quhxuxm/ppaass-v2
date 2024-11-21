use ppaass_crypto::error::CryptoError;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum CodecError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error(transparent)]
    Bincode(#[from] bincode::Error),
}