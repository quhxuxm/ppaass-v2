use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub enum Encryption {
    #[default]
    Plain,
    Aes(Vec<u8>),
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SessionInitRequest {
    pub agent_encryption: Encryption,
    pub auth_token: String,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum SessionInitResponseStatus {
    Success,
    Failure,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SessionInitResponse {
    pub proxy_encryption: Encryption,
    pub session_token: String,
    pub status: SessionInitResponseStatus,
}

