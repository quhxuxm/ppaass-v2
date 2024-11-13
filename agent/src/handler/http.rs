use crate::bo::config::Config;
use crate::error::AgentError;
use crate::handler::HandlerRequest;
use bytecodec::bytes::RemainingBytesDecoder;
use httpcodec::{BodyDecoder, RequestDecoder};
use std::sync::Arc;
pub async fn handle_http_client_tcp_stream(
    config: Arc<Config>,
    request: HandlerRequest,
) -> Result<(), AgentError> {
    let HandlerRequest {
        client_tcp_stream,
        session_token,
        agent_encryption,
        proxy_encryption,
        http_client,
        client_socket_addr
    } = request;
    let http_request_decoder = RequestDecoder::new(BodyDecoder::new(RemainingBytesDecoder::new()));
    client_tcp_stream.peek()
    todo!()
}
