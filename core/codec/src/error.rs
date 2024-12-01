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
    #[error("Invalid relay type byte: {0}")]
    InvalidRelayResponseStatusByte(u8),
    #[error("Invalid relay type byte: {0}")]
    InvalidRelayTypeByte(u8),
    #[error("Invalid agent packet byte: {0}")]
    InvalidAgentPacketByte(u8),
    #[error("Not enough remaining bytes: {0}")]
    NotEnoughRemainingBytes(u64),
    #[error("Can not found encryption with key: {0}")]
    EncryptionNotExist(String),
    #[error("Fail to get encryption holder lock")]
    EncryptionHolderLock,
}
