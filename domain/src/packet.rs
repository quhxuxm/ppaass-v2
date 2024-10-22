use accessory::Accessors;
use bytes::Bytes;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub enum PpaassPacketEncryption {
    #[default]
    Plain,
    Aes(Bytes),
}

#[derive(Deserialize, Serialize, Debug, Clone, Builder, Default, Accessors)]
pub struct PpaassPacket {
    #[access(get)]
    packet_id: String,
    #[access(get)]
    auth_token: String,
    #[access(get)]
    encryption: PpaassPacketEncryption,
    #[access(get)]
    payload: Bytes,
}
