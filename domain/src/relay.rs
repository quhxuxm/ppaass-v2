use crate::address::UnifiedAddress;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum RelayType {
    Tcp,
    Udp,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RelayRequest {
    pub session_token: String,
    pub relay_type: RelayType,
    pub content: RelayRequestContent,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RelayRequestContent {
    pub dst_address: UnifiedAddress,
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
    pub session_token: String,
    pub relay_type: RelayType,
    pub content: RelayResponseContent,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RelayResponseContent {
    pub dst_address: UnifiedAddress,
    pub payload: Vec<u8>,
}
