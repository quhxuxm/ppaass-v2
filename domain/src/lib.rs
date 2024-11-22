use crate::relay::{RelayRequest, RelayResponse};
use crate::session::{SessionInitRequest, SessionInitResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub mod address;
pub mod error;
pub mod relay;
pub mod session;
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string().replace("-", "").to_uppercase()
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AgentPacket {
    SessionInit(SessionInitRequest),
    Relay(RelayRequest),
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ProxyPacket {
    SessionInit(SessionInitResponse),
    Relay(RelayResponse),
}