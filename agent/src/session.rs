use ppaass_domain::tunnel::Encryption;
pub struct AgentSession {
    pub agent_encryption: Encryption,
    pub proxy_encryption: Encryption,
    pub session_token: String,
}
