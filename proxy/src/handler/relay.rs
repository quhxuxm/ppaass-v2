use crate::bo::state::ServerState;
use crate::destination::read::DestinationTransportRead;
use crate::destination::write::DestinationTransportWrite;
use crate::destination::DestinationTransport;
use crate::error::ProxyError;
use axum::extract::ws::{CloseFrame, Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::Response;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use bytes::BytesMut;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use ppaass_crypto::aes::{decrypt_with_aes, encrypt_with_aes};
use ppaass_domain::relay::{RelayInfo, RelayType};
use ppaass_domain::session::Encryption;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::{debug, error};
struct RelayAgentToDestValues {
    agent_encryption: Encryption,
    ws_read: SplitStream<WebSocket>,
    dest_transport_write: DestinationTransportWrite,
    session_token: String,
    relay_info_token: String,
}
async fn relay_agent_to_dest(
    RelayAgentToDestValues {
        agent_encryption,
        mut ws_read,
        mut dest_transport_write,
        session_token,
        relay_info_token,
    }: RelayAgentToDestValues,
) {
    loop {
        let agent_data = ws_read.next().await;
        let agent_data = match agent_data {
            None => {
                if let Err(e) = dest_transport_write.close().await {
                    error!(
                        session_token = { &session_token },
                        relay_info_token = { &relay_info_token },
                        "Fail to close destination write half on agent data exhausted: {e:?}");
                }
                return;
            }
            Some(Err(e)) => {
                error!(
                    session_token = { &session_token },
                    relay_info_token = { &relay_info_token },
                    "Fail to read client data: {e:?}"
                );
                return;
            }
            Some(Ok(agent_data)) => agent_data,
        };
        let agent_data = match agent_data {
            Message::Text(text_message) => {
                debug!(
                    session_token = { &session_token },
                    relay_info_token = { &relay_info_token },
                    "Received text message from agent: {text_message}"
                );
                continue;
            }
            Message::Binary(agent_data) => agent_data,
            Message::Ping(ping_data) => {
                debug!(
                    session_token = { &session_token },
                    relay_info_token = { &relay_info_token },
                    "Received ping message from agent:\n{}",
                    pretty_hex::pretty_hex(&ping_data)
                );
                continue;
            }
            Message::Pong(pong_data) => {
                debug!(
                    session_token = { &session_token },
                    relay_info_token = { &relay_info_token },
                    "Received pong message from agent:\n{}",
                    pretty_hex::pretty_hex(&pong_data)
                );
                continue;
            }
            Message::Close(Some(CloseFrame { code, reason })) => {
                debug!(
                    session_token = { &session_token },
                    relay_info_token = { &relay_info_token },
                    "Received close message from agent with code: {code}, reason: {reason}"
                );
                if let Err(e) = dest_transport_write.close().await {
                    error!(
                        session_token = { &session_token },
                        relay_info_token = { &relay_info_token },
                        "Fail to close destination write half on agent websocket closed: {e:?}");
                }
                return;
            }
            Message::Close(None) => {
                debug!(
                    session_token = { &session_token },
                    relay_info_token = { &relay_info_token },
                    "Received close message from agent without any information."
                );
                if let Err(e) = dest_transport_write.close().await {
                    error!(
                        session_token = { &session_token },
                        relay_info_token = { &relay_info_token },
                        "Fail to close destination write half on agent websocket closed: {e:?}");
                }
                return;
            }
        };
        let decrypted_agent_data = match &agent_encryption {
            Encryption::Plain => agent_data,
            Encryption::Aes(aes_token) => match decrypt_with_aes(aes_token, &agent_data) {
                Ok(decrypted_agent_data) => decrypted_agent_data,
                Err(e) => {
                    error!(
                        session_token = { &session_token },
                        relay_info_token = { &relay_info_token },
                        "Fail to aes decrypt agent data: {e:?}"
                    );
                    if let Err(e) = dest_transport_write.close().await {
                        error!(
                        session_token = { &session_token },
                        relay_info_token = { &relay_info_token },
                        "Fail to close destination write half on decrypt agent data with aes: {e:?}");
                    }
                    return;
                }
            },
        };
        let decrypted_agent_data = BytesMut::from(decrypted_agent_data.as_slice());
        if let Err(e) = dest_transport_write.send(decrypted_agent_data).await {
            error!(
                session_token = { &session_token },
                relay_info_token = { &relay_info_token },
                "Fail to send agent data to destination: {e:?}"
            );
            if let Err(e) = dest_transport_write.close().await {
                error!(
                        session_token = { &session_token },
                        relay_info_token = { &relay_info_token },
                        "Fail to close destination write half on fail to send data to destination: {e:?}");
            }
            return;
        }
    }
}
struct RelayDestToAgentValues {
    proxy_encryption: Encryption,
    ws_write: SplitSink<WebSocket, Message>,
    dest_transport_read: DestinationTransportRead,
    session_token: String,
    relay_info_token: String,
}
async fn relay_dest_to_agent(
    RelayDestToAgentValues {
        proxy_encryption,
        mut ws_write,
        mut dest_transport_read,
        session_token,
        relay_info_token,
    }: RelayDestToAgentValues,
) {
    loop {
        let dest_data = dest_transport_read.next().await;
        let dest_data = match dest_data {
            None => {
                if let Err(e) = ws_write.close().await {
                    error!(
                        session_token = { &session_token },
                        relay_info_token = { &relay_info_token },
                        "Fail to close agent websocket on destination exhausted: {e:?}");
                }
                return;
            }
            Some(Err(e)) => {
                error!(
                    session_token = { &session_token },
                    relay_info_token = { &relay_info_token },
                    "Fail to receive destination data: {e:?}"
                );
                if let Err(e) = ws_write.close().await {
                    error!(
                        session_token = { &session_token },
                        relay_info_token = { &relay_info_token },
                        "Fail to close agent websocket on read destination fail: {e:?}");
                }
                return;
            }
            Some(Ok(dest_data)) => dest_data,
        };
        let dest_data = match &proxy_encryption {
            Encryption::Plain => dest_data.into(),
            Encryption::Aes(aes_token) => match encrypt_with_aes(aes_token, &dest_data) {
                Ok(dest_data) => dest_data,
                Err(e) => {
                    error!(
                        session_token = { &session_token },
                        relay_info_token = { &relay_info_token },
                        "Fail to aes encrypt destination data: {e:?}"
                    );
                    if let Err(e) = ws_write.close().await {
                        error!(
                        session_token = { &session_token },
                        relay_info_token = { &relay_info_token },
                        "Fail to close agent websocket on aes encrypt destination data fail: {e:?}");
                    }
                    return;
                }
            },
        };
        let dest_data = Message::Binary(dest_data);
        if let Err(e) = ws_write.send(dest_data).await {
            error!(
                session_token = { &session_token },
                relay_info_token = { &relay_info_token },
                "Fail to send destination data to agent: {e:?}"
            );
            if let Err(e) = ws_write.close().await {
                error!(
                        session_token = { &session_token },
                        relay_info_token = { &relay_info_token },
                        "Fail to close agent websocket on write to agent websocket fail: {e:?}");
            }
            return;
        }
    }
}
pub async fn relay(
    Path((session_token, relay_info_token)): Path<(String, String)>,
    ws_upgrade: WebSocketUpgrade,
    State(server_state): State<Arc<ServerState>>,
) -> Response {
    debug!(
        session_token = { &session_token },
        relay_info_token = { &relay_info_token },
        "Receive websocket upgrade request."
    );
    ws_upgrade.on_upgrade(|ws| async move {
        let (agent_encryption, proxy_encryption) = {
            let session_repository = server_state.session_repository();
            let session_repository = match session_repository.lock() {
                Ok(session_repository) => session_repository,
                Err(_) => {
                    error!(
                        session_token = { &session_token },
                        relay_info_token = { &relay_info_token },
                        "Fail to acquire session repository lock for session"
                    );
                    return;
                }
            };
            let Some(session) = session_repository.get(&session_token) else {
                return;
            };
            (
                session.agent_encryption().clone(),
                session.proxy_encryption().clone(),
            )
        };
        let relay_info = match parse_relay_info(relay_info_token.clone(), &agent_encryption) {
            Ok(relay_info) => relay_info,
            Err(e) => {
                error!(
                    session_token = { &session_token },
                    relay_info_token = { &relay_info_token },
                    "Fail to parse relay info: {}",
                    e
                );
                return;
            }
        };
        let dst_address = relay_info.dst_address().clone();
        let dst_addresses: Vec<SocketAddr> = match dst_address.try_into() {
            Ok(dst_addresses) => dst_addresses,
            Err(e) => {
                error!(
                    session_token = { &session_token },
                    relay_info_token = { &relay_info_token },
                    "Fail to parse destination address to socket address: {}",
                    e
                );
                return;
            }
        };
        let dest_transport = match relay_info.relay_type() {
            RelayType::Tcp => match TcpStream::connect(dst_addresses.as_slice()).await {
                Ok(tcp_stream) => DestinationTransport::new_tcp(tcp_stream),
                Err(e) => {
                    error!(
                        session_token = { &session_token },
                        relay_info_token = { &relay_info_token },
                        "Fail to connect destination: {}",
                        e
                    );
                    return;
                }
            },
            RelayType::Udp => {
                todo!()
            }
        };
        let (dest_transport_write, dest_transport_read) = dest_transport.split();
        let (ws_write, ws_read) = ws.split();
        {
            let session_token = session_token.clone();
            let relay_info_token = relay_info_token.clone();
            tokio::spawn(relay_agent_to_dest(RelayAgentToDestValues {
                agent_encryption,
                ws_read,
                dest_transport_write,
                session_token,
                relay_info_token,
            }));
        }
        tokio::spawn(relay_dest_to_agent(RelayDestToAgentValues {
            proxy_encryption,
            ws_write,
            dest_transport_read,
            session_token,
            relay_info_token,
        }));
    })
}
fn parse_relay_info(
    relay_info: String,
    agent_encryption: &Encryption,
) -> Result<RelayInfo, ProxyError> {
    let relay_info = hex::decode(relay_info.to_lowercase())?;
    let relay_info = BASE64_STANDARD.decode(relay_info)?;
    let relay_info = match &agent_encryption {
        Encryption::Plain => relay_info,
        Encryption::Aes(aes_token) => decrypt_with_aes(aes_token, &relay_info)?,
    };
    let relay_info: RelayInfo = relay_info.try_into()?;
    Ok(relay_info)
}
