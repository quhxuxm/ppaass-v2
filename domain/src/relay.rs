use crate::address::UnifiedAddress;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum RelayType {
    Tcp,
    Udp,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RelayRequest {
    pub dst_address: UnifiedAddress,
    pub relay_type: RelayType,
    pub session_token: String,
    pub payload: Vec<u8>,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum RelayResponseStatus {
    Success,
    SessionNotFound,
    DestinationUnreachable,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RelayResponse {
    pub status: RelayResponseStatus,
    pub payload: Vec<u8>,
}
