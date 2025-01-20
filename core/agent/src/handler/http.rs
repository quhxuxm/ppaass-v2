use crate::bo::state::ServerState;
use crate::error::AgentError;
use crate::handler::{relay, tunnel_init, RelayRequest, TunnelInitHandlerResponse};
use bytecodec::bytes::{BytesEncoder, RemainingBytesDecoder};
use bytecodec::io::IoDecodeExt;
use bytecodec::EncodeExt;
use bytes::{Buf, Bytes};
use httpcodec::{
    BodyDecoder, BodyEncoder, HeaderField, HttpVersion, ReasonPhrase, Request, RequestDecoder,
    RequestEncoder, Response, ResponseEncoder, StatusCode,
};
use ppaass_domain::address::UnifiedAddress;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error};
use url::Url;
const CONNECT_METHOD: &str = "connect";
const OK_CODE: u16 = 200;
const CONNECTION_ESTABLISHED: &str = "Connection Established";
const HTTPS_PORT: u16 = 443;
const HTTP_PORT: u16 = 80;
const HTTP_REQUEST_BUF_LEN: usize = 512;
pub async fn handle_http_client_tcp_stream(
    mut client_tcp_stream: TcpStream,
    server_state: ServerState,
) -> Result<(), AgentError> {
    let mut request_decode_buf = Vec::new();
    loop {
        let mut request_stream_read_buf = [0u8; HTTP_REQUEST_BUF_LEN];
        let read_amount = client_tcp_stream.read(&mut request_stream_read_buf).await?;
        if 0 == read_amount {
            break;
        }
        request_decode_buf.extend(&request_stream_read_buf[..read_amount]);
        if read_amount < HTTP_REQUEST_BUF_LEN {
            break;
        }
    }
    let mut request_decoder: RequestDecoder<BodyDecoder<RemainingBytesDecoder>> =
        Default::default();
    let request_decode_buf_read = request_decode_buf.reader();
    let client_request = request_decoder.decode_exact(request_decode_buf_read)?;
    let (client_request, client_body) = client_request.take_body();
    let client_request_method = client_request.method();
    let client_request_target = client_request.request_target();
    let client_http_version = client_request.http_version();
    let client_request_header = client_request.header();
    let client_request_header_fields = client_request_header.fields();

    let mut proxy_request = Request::new(
        client_request_method,
        client_request_target,
        client_http_version,
        client_body,
    );
    client_request_header_fields.for_each(|header_field| {
        if header_field.name().eq_ignore_ascii_case("Proxy-Connection") {
            let connection_field = match HeaderField::new("Connection", header_field.value()) {
                Ok(connection_field) => connection_field,
                Err(e) => {
                    error!("Fail to add Connection field: {}", e);
                    return;
                }
            };
            proxy_request.header_mut().add_field(connection_field);
            return;
        }
        proxy_request.header_mut().add_field(header_field);
    });

    let request_method = proxy_request.method().to_string();
    let (destination_address, initial_http_request_bytes) =
        if request_method.to_lowercase() == CONNECT_METHOD {
            let request_target = proxy_request.request_target();
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
            let request_target = proxy_request.request_target();
            let request_url = Url::parse(request_target.as_str())?;
            let mut http_data_encoder = RequestEncoder::<BodyEncoder<BytesEncoder>>::default();
            let initial_http_request_bytes: Bytes =
                http_data_encoder.encode_into_bytes(proxy_request)?.into();
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
    debug!(
        "HTTP proxy begin connect to remote: {}",
        destination_address
    );
    let TunnelInitHandlerResponse {
        proxy_tcp_stream,
        agent_encryption,
        proxy_encryption,
        destination_address,
    } = tunnel_init(destination_address, server_state.clone()).await?;
    debug!(
        "HTTP proxy connect to remote success: {}",
        destination_address
    );
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
    debug!("HTTP proxy begin to relay: {}", destination_address);
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
