use crate::bo::state::ServerState;
use crate::error::AgentError;
use crate::handler::{relay, tunnel_init, RelayRequest, TunnelInitHandlerResponse};
use bytecodec::bytes::{BytesEncoder, RemainingBytesDecoder};
use bytecodec::io::IoDecodeExt;
use bytecodec::{EncodeExt, ErrorKind};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures_util::{SinkExt, StreamExt};
use httpcodec::{
    BodyDecoder, BodyEncoder, HttpVersion, ReasonPhrase, Request, RequestDecoder, RequestEncoder,
    Response, ResponseEncoder, StatusCode,
};
use ppaass_domain::address::UnifiedAddress;
use tokio::net::TcpStream;
use tokio_util::codec::{Decoder, Encoder, Framed};
use tracing::{debug, error};
use url::Url;
const CONNECT_METHOD: &str = "connect";
const OK_CODE: u16 = 200;
const CONNECTION_ESTABLISHED: &str = "Connection Established";
const HTTPS_PORT: u16 = 443;
const HTTP_PORT: u16 = 80;

#[derive(Debug, Default)]
struct HttpCodec {
    request_decoder: RequestDecoder<BodyDecoder<RemainingBytesDecoder>>,
    response_encoder: ResponseEncoder<BodyEncoder<BytesEncoder>>,
}
impl HttpCodec {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Decoder for HttpCodec {
    type Item = Request<Vec<u8>>;
    type Error = AgentError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let decode_result = match self.request_decoder.decode_exact(src.chunk()) {
            Ok(decode_result) => decode_result,
            Err(e) => {
                return match e.kind() {
                    ErrorKind::IncompleteDecoding => Ok(None),
                    other_kind => {
                        error!("Http agent fail to decode because of error: {other_kind:?}");
                        Err(AgentError::ByteCodec(e))
                    }
                }
            }
        };
        Ok(Some(decode_result))
    }
}

impl Encoder<Response<Vec<u8>>> for HttpCodec {
    type Error = AgentError;

    fn encode(&mut self, item: Response<Vec<u8>>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let encode_result = self
            .response_encoder
            .encode_into_bytes(item)
            .map_err(|e| AgentError::ByteCodec(e))?;
        dst.put_slice(encode_result.as_slice());
        Ok(())
    }
}

pub async fn handle_http_client_tcp_stream(
    mut client_tcp_stream: TcpStream,
    server_state: ServerState,
) -> Result<(), AgentError> {
    let mut http_request_framed = Framed::new(&mut client_tcp_stream, HttpCodec::new());
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
        http_request_framed
            .send(http_connect_success_response)
            .await?;
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
