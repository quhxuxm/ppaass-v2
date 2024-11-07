use crate::address::UnifiedAddress;
use crate::error::DomainError;
use accessory::Accessors;
use bytes::Bytes;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum RelayType {
    Tcp,
    Udp,
}
#[derive(Deserialize, Serialize, Debug, Clone, Accessors, Builder)]
pub struct RelayInfo {
    #[access(get)]
    src_address: UnifiedAddress,
    #[access(get)]
    dst_address: UnifiedAddress,
    #[access(get)]
    relay_type: RelayType,
}

impl TryFrom<Bytes> for RelayInfo {
    type Error = DomainError;
    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        let result = bincode::deserialize::<RelayInfo>(&value)?;
        Ok(result)
    }
}

impl TryFrom<Vec<u8>> for RelayInfo {
    type Error = DomainError;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let result = bincode::deserialize::<RelayInfo>(&value)?;
        Ok(result)
    }
}

impl TryFrom<RelayInfo> for Bytes {
    type Error = DomainError;
    fn try_from(value: RelayInfo) -> Result<Self, Self::Error> {
        let result = bincode::serialize(&value)?;
        Ok(result.into())
    }
}
