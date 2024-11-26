use crate::bo::state::ServerState;
use crate::error::AgentError;
use crate::handler::{relay, tunnel_init, RelayRequest, TunnelInitHandlerResponse};
use bytecodec::bytes::{BytesEncoder, RemainingBytesDecoder};
use bytecodec::io::IoDecodeExt;
use bytecodec::{EncodeExt, ErrorKind};
use bytes::{Buf, Bytes, BytesMut};
use futures_util::StreamExt;
use httpcodec::{
    BodyDecoder, BodyEncoder, HttpVersion, ReasonPhrase, Request, RequestDecoder, RequestEncoder,
    Response, ResponseEncoder, StatusCode,
};
use ppaass_domain::address::UnifiedAddress;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_util::codec::{Decoder, Framed};
use tracing::{debug, error};
use url::Url;
const CONNECT_METHOD: &str = "connect";
const OK_CODE: u16 = 200;
const CONNECTION_ESTABLISHED: &str = "Connection Established";
const HTTPS_PORT: u16 = 443;
const HTTP_PORT: u16 = 80;
struct HttpRequestDecoder {
    http_request_decoder: RequestDecoder<BodyDecoder<RemainingBytesDecoder>>,
}
impl HttpRequestDecoder {
    pub fn new() -> Self {
        Self {
            http_request_decoder: RequestDecoder::default(),
        }
    }
}
impl Decoder for HttpRequestDecoder {
    type Item = Request<Vec<u8>>;
    type Error = AgentError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.http_request_decoder.decode_exact(src.chunk()) {
            Ok(http_request) => Ok(Some(http_request)),
            Err(e) => match e.kind() {
                ErrorKind::IncompleteDecoding => Ok(None),
                _ => {
                    error!("Failed to decode http request: {}", e);
                    Err(e.into())
                }
            },
        }
    }
}
pub async fn handle_http_client_tcp_stream(
    mut client_tcp_stream: TcpStream,
    server_state: ServerState,
) -> Result<(), AgentError> {
    let mut http_request_framed = Framed::new(&mut client_tcp_stream, HttpRequestDecoder::new());
    let http_request = http_request_framed
        .next()
        .await
        .ok_or(AgentError::ClientTcpConnectionExhausted)??;
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
