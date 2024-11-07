use accessory::Accessors;
use derive_builder::Builder;
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
}
