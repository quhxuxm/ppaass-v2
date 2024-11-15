use crate::bo::config::Config;
use crate::error::AgentError;
use crate::handler::{generate_relay_websocket, HandlerRequest};
use bytecodec::bytes::RemainingBytesDecoder;
use bytecodec::io::IoDecodeExt;
use bytecodec::ErrorKind;
use bytes::Buf;
use httpcodec::{BodyDecoder, Request, RequestDecoder};
use ppaass_domain::address::UnifiedAddress;
use ppaass_domain::relay::{RelayInfoBuilder, RelayType};
use reqwest::Url;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tracing::debug;
const CONNECT_METHOD: &str = "connect";
async fn parse_http_request(mut client_tcp_stream: TcpStream) -> Result<Request<Vec<u8>>, AgentError> {
    let mut http_request_decoder = RequestDecoder::new(BodyDecoder::new(RemainingBytesDecoder::new()));
    let mut buffer_size = 65536;
    loop {
        let mut initial_request_buf = vec![0u8; buffer_size];
        let size = client_tcp_stream.peek(&mut initial_request_buf).await?;
        let request_buf = initial_request_buf[..size].to_vec();
        let mut request_buf_reader = request_buf.reader();
        return match http_request_decoder.decode_exact(&mut request_buf_reader) {
            Ok(http_request) => {
                let mut _advance_buf = vec![0u8; size];
                client_tcp_stream.read_exact(&mut _advance_buf).await?;
                Ok(http_request)
            }
            Err(e) => {
                if e.kind() == &ErrorKind::IncompleteDecoding {
                    buffer_size *= 2;
                    continue;
                }
                Err(e.into())
            }
        };
    }
}
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
    let http_request = parse_http_request(client_tcp_stream).await?;
    let request_target = http_request.request_target();
    let request_url = Url::parse(request_target.as_str())?;
    debug!("Receive http request: {}",request_url);
    let mut relay_info_builder = RelayInfoBuilder::default();
    let mut relay_info_builder = relay_info_builder.relay_type(RelayType::Tcp).src_address(client_socket_addr.into());
    let request_method = http_request.method().to_string();
    if request_method == CONNECT_METHOD {
        //HTTPS request with proxy
        relay_info_builder = relay_info_builder.dst_address(UnifiedAddress::Domain {
            host: request_url.host().ok_or(AgentError::UnknownHostFromTargetUrl(request_url.to_string()))?.to_string(),
            port: request_url.port().unwrap_or(443),
        });
    } else {
        //HTTP request with proxy
        relay_info_builder = relay_info_builder.dst_address(UnifiedAddress::Domain {
            host: request_url.host().ok_or(AgentError::UnknownHostFromTargetUrl(request_url.to_string()))?.to_string(),
            port: request_url.port().unwrap_or(80),
        });
    }
    let relay_info = relay_info_builder.build()?;
    let (relay_ws, relay_info_token) = generate_relay_websocket(&session_token, relay_info, &agent_encryption, &config, &http_client).await?;
    Ok(())
}
