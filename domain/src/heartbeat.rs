use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct HeartbeatPing {
    pub heartbeat_time: DateTime<Utc>,
}
impl Default for HeartbeatPing {
    fn default() -> Self {
        Self {
            heartbeat_time: Utc::now(),
        }
    }
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct HeartbeatPong {
    pub heartbeat_time: DateTime<Utc>,
}
impl Default for HeartbeatPong {
    fn default() -> Self {
        Self {
            heartbeat_time: Utc::now(),
        }
    }
}
