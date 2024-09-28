use crate::error::DomainError;
use crate::message::PpaassMessagePacket;
use bytes::Bytes;
use derive_more::Constructor;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
pub mod business;
mod error;
pub mod message;

#[derive(Constructor)]
pub struct DecryptedPpaassMessagePacket<T> {
    message_id: String,
    user_token: Bytes,
    business_data: T,
}

impl<T> DecryptedPpaassMessagePacket<T> {
    pub fn message_id(&self) -> &str {
        &self.message_id
    }

    pub fn user_token(&self) -> &[u8] {
        &self.user_token
    }

    pub fn business_data(&self) -> &T {
        &self.business_data
    }
}

pub fn decrypt_message_packet<T: DeserializeOwned>(
    msg: PpaassMessagePacket,
) -> Result<DecryptedPpaassMessagePacket<T>, DomainError> {
    let message_id = msg.message_id().to_owned();
    let user_token: Bytes = msg.user_token().to_vec().into();
    let payload = msg.payload();
    let payload_encryption = payload.encryption().clone();
    let business_data_bytes = payload.business_data();
    let business_data = bincode::deserialize::<T>(&business_data_bytes)?;
    Ok(DecryptedPpaassMessagePacket::new(
        message_id,
        user_token,
        business_data,
    ))
}

pub fn encrypt_message_packet<T: Serialize>(
    msg: PpaassMessagePacket,
) -> Result<DecryptedPpaassMessagePacket<T>, DomainError> {
    let business_data = bincode::serialize::<T>(&msg)?;
}
