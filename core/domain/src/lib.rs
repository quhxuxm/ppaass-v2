use crate::address::UnifiedAddress;
use crate::heartbeat::{HeartbeatPing, HeartbeatPong};
use crate::tunnel::{TunnelInitRequest, TunnelInitResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub mod address;
pub mod error;
pub mod heartbeat;
pub mod tunnel;
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string().replace("-", "").to_uppercase()
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum AgentControlPacket {
    TunnelInit(TunnelInitRequest),
    Heartbeat(HeartbeatPing),
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ProxyControlPacket {
    TunnelInit((String, TunnelInitResponse)),
    Heartbeat(HeartbeatPong),
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum AgentDataPacket {
    Tcp(Vec<u8>),
    Udp {
        destination_address: UnifiedAddress,
        payload: Vec<u8>,
    },
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ProxyDataPacket {
    Tcp(Vec<u8>),
    Udp {
        destination_address: UnifiedAddress,
        payload: Vec<u8>,
    },
}
