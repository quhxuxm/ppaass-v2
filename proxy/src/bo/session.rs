use accessory::Accessors;
use chrono::{DateTime, Utc};
use derive_builder::Builder;
use ppaass_domain::relay::RelayInfo;
use ppaass_domain::session::Encryption;
#[derive(Debug, Clone, Accessors, Builder)]
pub struct Session {
    #[access(get(ty(&str)))]
    session_token: String,
    #[access(get(ty(&str)))]
    auth_token: String,
    #[access(get)]
    agent_encryption: Encryption,
    #[access(get)]
    proxy_encryption: Encryption,
    #[access(get, get_mut)]
    relays: Vec<RelayInfo>,
    #[access(get)]
    create_time: DateTime<Utc>,
    #[access(get, set)]
    update_time: DateTime<Utc>,
}
