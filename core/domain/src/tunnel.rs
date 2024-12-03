use crate::address::UnifiedAddress;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub enum Encryption {
    #[default]
    Plain,
    Aes(Vec<u8>),
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum TunnelType {
    Tcp { keepalive: bool },
    Udp,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TunnelInitRequest {
    pub agent_encryption: Encryption,
    pub auth_token: String,
    pub dst_address: UnifiedAddress,
    pub tunnel_type: TunnelType,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TunnelInitResponse {
    pub proxy_encryption: Encryption,
}
