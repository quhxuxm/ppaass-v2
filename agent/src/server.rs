use std::sync::Arc;
use crate::bo::config::Config;
use crate::error::AgentError;
pub struct AgentServer{
    config: Arc<Config>
}
impl AgentServer {
    pub fn new(config: Arc<Config>) -> Result<Self, AgentError> {
        Ok(Self {
            config
        })
    }
    
    pub async fn start(self) -> Result<(), AgentError> {}
}