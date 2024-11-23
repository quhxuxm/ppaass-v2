use crate::bo::state::ServerState;
use ppaass_codec::error::CodecError;
use ppaass_codec::EncryptionHolder;
use ppaass_domain::tunnel::Encryption;
use std::sync::Arc;
pub struct AgentEncryptionHolder {
    server_state: Arc<ServerState>,
}
impl AgentEncryptionHolder {
    pub fn new(server_state: Arc<ServerState>) -> Self {
        AgentEncryptionHolder { server_state }
    }
}
impl EncryptionHolder for AgentEncryptionHolder {
    fn get_encryption(&self, encryption_key: impl AsRef<str>) -> Result<Option<Arc<Encryption>>, CodecError> {
        let session_repository = self.server_state.session_repository().lock().map_err(|_| CodecError::EncryptionHolderLock)?;
        let session = session_repository.get(encryption_key.as_ref());
        match session {
            None => Ok(None),
            Some(session) => {
                Ok(Some(session.agent_encryption().clone()))
            }
        }
    }
}
pub struct ProxyEncryptionHolder {
    server_state: Arc<ServerState>,
}
impl ProxyEncryptionHolder {
    pub fn new(server_state: Arc<ServerState>) -> Self {
        ProxyEncryptionHolder { server_state }
    }
}
impl EncryptionHolder for ProxyEncryptionHolder {
    fn get_encryption(&self, encryption_key: impl AsRef<str>) -> Result<Option<Arc<Encryption>>, CodecError> {
        let session_repository = self.server_state.session_repository().lock().map_err(|_| CodecError::EncryptionHolderLock)?;
        let session = session_repository.get(encryption_key.as_ref());
        match session {
            None => Ok(None),
            Some(session) => {
                Ok(Some(session.proxy_encryption().clone()))
            }
        }
    }
}