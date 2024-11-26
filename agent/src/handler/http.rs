use crate::bo::state::ServerState;
use crate::error::AgentError;
use crate::handler::{relay, tunnel_init, RelayRequest, TunnelInitHandlerResponse};
use bytecodec::bytes::{BytesEncoder, RemainingBytesDecoder};
use bytecodec::io::IoDecodeExt;
use bytecodec::{EncodeExt, ErrorKind};
use bytes::{Buf, Bytes};
use httpcodec::{
    BodyDecoder, BodyEncoder, HttpVersion, ReasonPhrase, Request, RequestDecoder, RequestEncoder,
    Response, ResponseEncoder, StatusCode,
};
use ppaass_domain::address::UnifiedAddress;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::debug;
use url::Url;
const CONNECT_METHOD: &str = "connect";
const OK_CODE: u16 = 200;
const CONNECTION_ESTABLISHED: &str = "Connection Established";
const HTTPS_PORT: u16 = 443;
const HTTP_PORT: u16 = 80;
async fn parse_http_request(
    client_tcp_stream: &mut TcpStream,
) -> Result<Request<Vec<u8>>, AgentError> {
    let mut http_request_decoder =
        RequestDecoder::new(BodyDecoder::new(RemainingBytesDecoder::new()));
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
    mut client_tcp_stream: TcpStream,
    server_state: ServerState,
) -> Result<(), AgentError> {
    let http_request = parse_http_request(&mut client_tcp_stream).await?;
    let request_method = http_request.method().to_string();
    let (destination_address, initial_http_request_bytes) =
        if request_method.to_lowercase() == CONNECT_METHOD {
            let request_target = http_request.request_target();
            let request_url = Url::parse(format!("https://{}", request_target.as_str()).as_str())?;
            debug!("Receive https request: {}", request_url);
            //HTTPS request with proxy
            (
                UnifiedAddress::Domain {
                    host: request_url
                        .host_str()
                        .ok_or(AgentError::UnknownHostFromTargetUrl(
                            request_url.to_string(),
                        ))?
                        .to_string(),
                    port: request_url.port().unwrap_or(HTTPS_PORT),
                },
                None,
            )
        } else {
            //HTTP request with proxy
            let request_target = http_request.request_target();
            let request_url = Url::parse(request_target.as_str())?;
            debug!("Receive http request: {}", request_url);
            let mut http_data_encoder = RequestEncoder::<BodyEncoder<BytesEncoder>>::default();
            let initial_http_request_bytes: Bytes =
                http_data_encoder.encode_into_bytes(http_request)?.into();
            (
                UnifiedAddress::Domain {
                    host: request_url
                        .host_str()
                        .ok_or(AgentError::UnknownHostFromTargetUrl(
                            request_url.to_string(),
                        ))?
                        .to_string(),
                    port: request_url.port().unwrap_or(HTTP_PORT),
                },
                Some(initial_http_request_bytes),
            )
        };
    let TunnelInitHandlerResponse {
        proxy_tcp_stream,
        agent_encryption,
        proxy_encryption,
        destination_address,
    } = tunnel_init(destination_address, server_state.clone()).await?;
    if initial_http_request_bytes.is_none() {
        //For https proxy
        let http_connect_success_response = Response::new(
            HttpVersion::V1_1,
            StatusCode::new(OK_CODE)?,
            ReasonPhrase::new(CONNECTION_ESTABLISHED)?,
            vec![],
        );
        let mut http_connect_success_response_encoder =
            ResponseEncoder::<BodyEncoder<BytesEncoder>>::default();
        let response_bytes = http_connect_success_response_encoder
            .encode_into_bytes(http_connect_success_response)?;
        client_tcp_stream.write_all(&response_bytes).await?;
    }
    relay(
        RelayRequest {
            client_tcp_stream,
            proxy_tcp_stream,
            agent_encryption,
            proxy_encryption,
            init_data: initial_http_request_bytes,
            destination_address,
        },
        server_state,
    )
        .await?;
    Ok(())
}
