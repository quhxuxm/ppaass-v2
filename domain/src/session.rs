use crate::relay::RelayInfo;
use accessory::Accessors;
use bytes::Bytes;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub enum Encryption {
    #[default]
    Plain,
    Aes(Bytes),
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CreateSessionRequest {
    pub agent_encryption: Encryption,
    pub auth_token: String,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CreateSessionResponse {
    pub proxy_encryption: Encryption,
    pub session_token: String,
}
#[derive(Deserialize, Serialize, Debug, Clone, Accessors, Builder)]
pub struct GetSessionResponse {
    #[access(get(ty(&str)))]
    session_token: String,
    #[access(get(ty(&str)))]
    auth_token: String,
    #[access(get)]
    relay_infos: Vec<RelayInfo>,
}
#[derive(Deserialize, Serialize, Debug, Clone, Accessors, Builder)]
pub struct RefreshSessionRequest {
    #[access(get)]
    agent_encryption: Encryption,
    #[access(get(ty(&str)))]
    auth_token: String,
    #[access(get(ty(&str)))]
    previous_session_token: String,
}
#[derive(Deserialize, Serialize, Debug, Clone, Accessors, Builder)]
pub struct RefreshSessionResponse {
    #[access(get)]
    proxy_encryption: Encryption,
    #[access(get(ty(&str)))]
    session_token: String,
}
