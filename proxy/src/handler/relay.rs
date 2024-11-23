use crate::bo::state::ServerState;
use crate::destination::DestinationTransport;
use crate::error::ProxyError;
use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;
use ppaass_crypto::aes::{decrypt_with_aes, encrypt_with_aes};
use ppaass_domain::tunnel::Encryption;
use tokio::net::TcpStream;
use tokio_stream::StreamExt as TokioStreamExt;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::error;
pub struct RelayStartRequest {
    pub agent_encryption: Encryption,
    pub proxy_encryption: Encryption,
    pub destination_transport: DestinationTransport,
}
pub async fn start_relay(
    agent_tcp_stream: TcpStream,
    relay_start_request: RelayStartRequest,
    server_state: ServerState,
) -> Result<(), ProxyError> {
    let RelayStartRequest {
        agent_encryption,
        proxy_encryption,
        destination_transport,
    } = relay_start_request;
    let agent_tcp_framed = Framed::with_capacity(
        agent_tcp_stream,
        LengthDelimitedCodec::new(),
        *server_state.config().agent_buffer_size(),
    );
    let (destination_transport_tx, destination_transport_rx) = destination_transport.split();
    let (agent_tcp_framed_tx, agent_tcp_framed_rx) = agent_tcp_framed.split();
    let decrypted_agent_stream = agent_tcp_framed_rx.map_while(move |agent_read_item| {
        let agent_data = match agent_read_item {
            Ok(agent_data) => agent_data,
            Err(e) => {
                error!("Failed to read agent data: {}", e);
                return Some(Err(ProxyError::Io(e)));
            }
        };
        match &agent_encryption {
            Encryption::Plain => Some(Ok(agent_data)),
            Encryption::Aes(aes_token) => match decrypt_with_aes(&aes_token, &agent_data) {
                Ok(decrypted_agent_data) => Some(Ok(BytesMut::from_iter(decrypted_agent_data))),
                Err(e) => {
                    error!("Fail to decrypt agent data: {e:?}");
                    return Some(Err(e.into()));
                }
            },
        }
    });
    let encrypted_destination_stream =
        destination_transport_rx.map_while(move |destination_item| {
            let destination_data = match destination_item {
                Ok(destination_data) => destination_data.freeze(),
                Err(e) => {
                    error!("Failed to read destination data: {e:?}");
                    return Some(Err(e.into()));
                }
            };
            match &proxy_encryption {
                Encryption::Plain => Some(Ok(destination_data)),
                Encryption::Aes(aes_token) => match encrypt_with_aes(&aes_token, &destination_data)
                {
                    Ok(encrypted_destination_data) => {
                        Some(Ok(Bytes::from(encrypted_destination_data)))
                    }
                    Err(e) => {
                        error!("Fail to encrypt destination data: {e:?}");
                        Some(Err(ProxyError::Crypto(e).into()))
                    }
                },
            }
        });
    tokio::spawn(decrypted_agent_stream.forward(destination_transport_tx));
    tokio::spawn(encrypted_destination_stream.forward(agent_tcp_framed_tx));
    Ok(())
}
