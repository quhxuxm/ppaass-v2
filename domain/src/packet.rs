use crate::error::DomainError;
use accessory::Accessors;
use bytes::{Bytes, BytesMut};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug, Clone, Builder, Default, Accessors)]
pub struct PpaassPacket {
    #[access(get(ty(&str)))]
    packet_id: String,
    #[access(get(ty(&str)))]
    auth_token: String,
    #[access(get(ty(&[u8])))]
    payload: Bytes,
}

impl TryFrom<PpaassPacket> for Bytes {
    type Error = DomainError;
    fn try_from(value: PpaassPacket) -> Result<Self, Self::Error> {
        let result = bincode::serialize(&value)?;
        Ok(Bytes::from(result))
    }
}

impl TryFrom<Bytes> for PpaassPacket {
    type Error = DomainError;
    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        let result = bincode::deserialize::<PpaassPacket>(&value)?;
        Ok(result)
    }
}

impl TryFrom<BytesMut> for PpaassPacket {
    type Error = DomainError;
    fn try_from(value: BytesMut) -> Result<Self, Self::Error> {
        let result = bincode::deserialize::<PpaassPacket>(&value)?;
        Ok(result)
    }
}
