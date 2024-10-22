use crate::address::UnifiedAddress;
use crate::error::DomainError;
use accessory::Accessors;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug, Clone, Accessors, Builder)]
pub struct KeyExchange {
    expire: DateTime<Utc>,
    encryption_key: Bytes,
}

impl TryFrom<Bytes> for KeyExchange {
    type Error = DomainError;
    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        let result = bincode::deserialize(&value)?;
        Ok(result)
    }
}

impl TryFrom<KeyExchange> for Bytes {
    type Error = DomainError;
    fn try_from(value: KeyExchange) -> Result<Self, Self::Error> {
        let result = bincode::serialize(&value)?;
        Ok(result.into())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Builder, Accessors)]
pub struct TransferData {
    #[access(get)]
    src_address: UnifiedAddress,
    #[access(get)]
    dest_address: UnifiedAddress,
    #[access(get)]
    data: Bytes,
}

impl TryFrom<Bytes> for TransferData {
    type Error = DomainError;
    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        let result = bincode::deserialize(&value)?;
        Ok(result)
    }
}

impl TryFrom<TransferData> for Bytes {
    type Error = DomainError;
    fn try_from(value: TransferData) -> Result<Self, Self::Error> {
        let result = bincode::serialize(&value)?;
        Ok(result.into())
    }
}
