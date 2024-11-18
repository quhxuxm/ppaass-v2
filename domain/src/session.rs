use crate::relay::RelayInfo;
use bytes::Bytes;
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
#[derive(Deserialize, Serialize, Debug, Clone, )]
pub struct GetSessionResponse {
    pub session_token: String,
    pub auth_token: String,
    pub relay_infos: Vec<RelayInfo>,
}

