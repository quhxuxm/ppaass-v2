use crate::bo::config::Config;
use crate::error::AgentError;
use crate::handler::HandlerRequest;
use std::sync::Arc;
pub async fn handle_http_client_tcp_stream(
    config: Arc<Config>,
    request: HandlerRequest,
) -> Result<(), AgentError> {
    todo!()
}
