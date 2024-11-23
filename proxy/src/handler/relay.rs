use crate::bo::state::ServerState;
use crate::destination::DestinationTransport;
use crate::error::ProxyError;
use bytes::BytesMut;
use futures_util::{StreamExt, TryStreamExt};
use ppaass_crypto::aes::decrypt_with_aes;
use ppaass_crypto::error::CryptoError;
use ppaass_domain::relay::{RelayRequest, RelayRequestContent, RelayType};
use ppaass_domain::tunnel::Encryption;
use std::sync::Arc;
use tokio::net::TcpStream;
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
    server_state: Arc<ServerState>,
) -> Result<(), ProxyError> {
    let RelayStartRequest { agent_encryption, proxy_encryption, destination_transport } = relay_start_request;
    let agent_tcp_framed = Framed::with_capacity(agent_tcp_stream, LengthDelimitedCodec::new(), *server_state.config().agent_buffer_size());
    let (agent_tcp_framed_tx, agent_tcp_framed_rx) = agent_tcp_framed.split();
    let stream = agent_tcp_framed_rx.try_filter_map(|agent_data| {
        let agent_encryption=agent_encryption.clone();
        async move {
            match agent_encryption {
                Encryption::Plain => Ok(Some(agent_data)),
                Encryption::Aes(aes_token) => {
                    let decrypted_agent_data = match decrypt_with_aes(&aes_token, &agent_data) {
                        Ok(decrypted_agent_data) => decrypted_agent_data,
                        Err(e) => {
                            error!("Fail to decrypt agent data: {e:?}");
                            return Ok(None);
                        }
                    };
                    Ok(Some(BytesMut::from_iter(decrypted_agent_data)))
                }
            }
        }
    });
    todo!()
}
