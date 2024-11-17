use ppaass_domain::session::Encryption;
pub struct AgentSession {
    pub agent_encryption: Encryption,
    pub proxy_encryption: Encryption,
    pub session_token: String,
}
