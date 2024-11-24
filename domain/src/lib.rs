use crate::heartbeat::{HeartbeatPing, HeartbeatPong};
use crate::tunnel::{TunnelInitRequest, TunnelInitResponse};
use uuid::Uuid;
pub mod address;
pub mod error;
pub mod heartbeat;
pub mod tunnel;
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string().replace("-", "").to_uppercase()
}

pub enum AgentPacket {
    TunnelInit(TunnelInitRequest),
    Heartbeat(HeartbeatPing),
    Relay(Vec<u8>),
}

pub enum ProxyPacket {
    TunnelInit((String, TunnelInitResponse)),
    Heartbeat(HeartbeatPong),
    Relay(Vec<u8>),
}
