use crate::bo::state::ServerState;
use crate::codec::ProxyConnectionCodec;
use crate::error::AgentError;
use bytes::{Bytes, BytesMut};
use futures_util::{SinkExt, StreamExt};
use ppaass_crypto::aes::{decrypt_with_aes, encrypt_with_aes};
use ppaass_crypto::random_32_bytes;
use ppaass_domain::address::UnifiedAddress;
use ppaass_domain::tunnel::{Encryption, TunnelInitRequest, TunnelInitResponse, TunnelType};
use tokio::net::TcpStream;
use tokio_stream::StreamExt as TokioStreamExt;
use tokio_util::codec::{BytesCodec, Framed, FramedParts, LengthDelimitedCodec};
use tracing::error;
pub mod http;
pub mod socks5;

pub struct TunnelInitHandlerResponse {
    proxy_tcp_stream: TcpStream,
    agent_encryption: Encryption,
    proxy_encryption: Encryption,
}

pub async fn tunnel_init(
    destination_address: UnifiedAddress,
    server_state: ServerState,
) -> Result<TunnelInitHandlerResponse, AgentError> {
    let proxy_tcp_stream = server_state
        .proxy_connection_pool()
        .take_proxy_connection()
        .await?;
    let mut proxy_tcp_framed = Framed::new(
        proxy_tcp_stream,
        ProxyConnectionCodec::new(
            server_state.config().auth_token().to_owned(),
            server_state.rsa_crypto_holder().clone(),
        ),
    );
    let agent_encryption = Encryption::Aes(random_32_bytes());
    proxy_tcp_framed
        .send(TunnelInitRequest {
            agent_encryption: agent_encryption.clone(),
            auth_token: server_state.config().auth_token().to_owned(),
            dst_address: destination_address,
            tunnel_type: TunnelType::Tcp,
        })
        .await?;
    let TunnelInitResponse { proxy_encryption } = StreamExt::next(&mut proxy_tcp_framed)
        .await
        .ok_or(AgentError::ProxyConnectionExhausted)??;
    let FramedParts {
        io: proxy_tcp_stream,
        ..
    } = proxy_tcp_framed.into_parts();
    Ok(TunnelInitHandlerResponse {
        proxy_tcp_stream,
        agent_encryption,
        proxy_encryption,
    })
}

pub struct RelayRequest {
    pub client_tcp_stream: TcpStream,
    pub proxy_tcp_stream: TcpStream,
    pub init_data: Option<Bytes>,
    pub agent_encryption: Encryption,
    pub proxy_encryption: Encryption,
}

pub async fn relay(
    relay_request: RelayRequest,
    server_state: ServerState,
) -> Result<(), AgentError> {
    let RelayRequest {
        client_tcp_stream,
        proxy_tcp_stream,
        init_data,
        agent_encryption,
        proxy_encryption,
    } = relay_request;
    let client_tcp_framed = Framed::with_capacity(
        client_tcp_stream,
        BytesCodec::new(),
        *server_state.config().client_relay_buffer_size() as usize,
    );
    let (mut client_tcp_framed_tx, client_tcp_framed_rx) = client_tcp_framed.split::<BytesMut>();
    let proxy_tcp_framed = Framed::with_capacity(
        proxy_tcp_stream,
        LengthDelimitedCodec::new(),
        *server_state.config().proxy_relay_buffer_size() as usize,
    );
    let (proxy_tcp_framed_tx, proxy_tcp_framed_rx) = proxy_tcp_framed.split();
    if let Some(init_data) = init_data {
        client_tcp_framed_tx
            .send(BytesMut::from(init_data.as_ref()))
            .await?;
    }
    let encrypted_client_stream = client_tcp_framed_rx.map_while(move |client_item| {
        let client_data = match client_item {
            Ok(client_data) => client_data.freeze(),
            Err(e) => {
                error!("Fail to read client data: {e:?}");
                return Some(Err(e.into()));
            }
        };
        match &agent_encryption {
            Encryption::Plain => Some(Ok(client_data)),
            Encryption::Aes(aes_token) => match encrypt_with_aes(&aes_token, &client_data) {
                Ok(encrypted_client_data) => Some(Ok(Bytes::from(encrypted_client_data))),
                Err(e) => {
                    error!("Fail to encrypt client data: {e:?}");
                    Some(Err(AgentError::Crypto(e).into()))
                }
            },
        }
    });
    let decrypted_proxy_stream = proxy_tcp_framed_rx.map_while(move |proxy_item| {
        let proxy_data = match proxy_item {
            Ok(proxy_data) => proxy_data,
            Err(e) => {
                error!("Failed to read proxy data: {}", e);
                return Some(Err(AgentError::Io(e).into()));
            }
        };
        match &proxy_encryption {
            Encryption::Plain => Some(Ok(proxy_data)),
            Encryption::Aes(aes_token) => match decrypt_with_aes(&aes_token, &proxy_data) {
                Ok(decrypted_proxy_data) => Some(Ok(BytesMut::from_iter(decrypted_proxy_data))),
                Err(e) => {
                    error!("Fail to decrypt proxy data: {e:?}");
                    return Some(Err(AgentError::Crypto(e).into()));
                }
            },
        }
    });
    tokio::spawn(encrypted_client_stream.forward(proxy_tcp_framed_tx));
    tokio::spawn(decrypted_proxy_stream.forward(client_tcp_framed_tx));
    Ok(())
}
