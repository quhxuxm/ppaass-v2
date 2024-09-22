use bytes::Bytes;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
pub enum PpaassMessagePayloadEncryption {
    Aes(Bytes),
}

#[derive(Serialize, Deserialize, Debug, Constructor)]
pub struct PpaassMessagePayload {
    encryption: PpaassMessagePayloadEncryption,
    encrypted_business_data: Bytes,
}

impl PpaassMessagePayload {
    pub fn encryption(&self) -> &PpaassMessagePayloadEncryption {
        &self.encryption
    }

    pub fn encrypted_business_data(&self) -> &[u8] {
        &self.encrypted_business_data
    }
}

#[derive(Serialize, Deserialize, Debug, Constructor)]
pub struct PpaassMessagePacket {
    message_id: String,
    payload: PpaassMessagePayload,
}

impl PpaassMessagePacket {
    pub fn message_id(&self) -> &str {
        &self.message_id
    }

    pub fn payload(&self) -> &PpaassMessagePayload {
        &self.payload
    }
}
