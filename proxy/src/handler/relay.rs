use crate::bo::state::ServerState;
use crate::destination::read::DestinationTransportRead;
use crate::destination::write::DestinationTransportWrite;
use crate::destination::DestinationTransport;
use crate::error::ProxyError;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::response::Response;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use bytes::BytesMut;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use ppaass_crypto::aes::{decrypt_with_aes, encrypt_with_aes};
use ppaass_domain::relay::{RelayInfo, RelayType};
use ppaass_domain::session::Encryption;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::{debug, error};
#[derive(Deserialize)]
pub struct RelayUpgradeParam {
    session_token: String,
}
async fn relay_agent_to_dest(
    agent_encryption: Encryption,
    mut ws_read: SplitStream<WebSocket>,
    mut dest_transport_write: DestinationTransportWrite,
) {
    loop {
        let agent_data = ws_read.next().await;
        let agent_data = match agent_data {
            None => return,
            Some(Err(e)) => return,
            Some(Ok(agent_data)) => agent_data,
        };
        if let Message::Binary(agent_data) = agent_data {
            let decrypted_agent_data = match &agent_encryption {
                Encryption::Plain => agent_data.into(),
                Encryption::Aes(aes_token) => {
                    let decrypted_agent_data = match decrypt_with_aes(aes_token, &agent_data) {
                        Ok(decrypted_agent_data) => decrypted_agent_data,
                        Err(e) => {
                            continue;
                        }
                    };
                    decrypted_agent_data
                }
            };
            let decrypted_agent_data = BytesMut::from(decrypted_agent_data.as_slice());
            if let Err(e) = dest_transport_write.send(decrypted_agent_data).await {
                return;
            }
        }
    }
}
async fn relay_dest_to_agent(
    proxy_encryption: Encryption,
    mut ws_write: SplitSink<WebSocket, Message>,
    mut dest_transport_read: DestinationTransportRead,
) {
    loop {
        let dest_data = dest_transport_read.next().await;
        let dest_data = match dest_data {
            None => return,
            Some(Err(e)) => return,
            Some(Ok(dest_data)) => dest_data,
        };
        let dest_data = match &proxy_encryption {
            Encryption::Plain => dest_data.into(),
            Encryption::Aes(aes_token) => match encrypt_with_aes(aes_token, &dest_data) {
                Ok(dest_data) => dest_data,
                Err(_) => {
                    return;
                }
            },
        };
        let dest_data = Message::Binary(dest_data);
        if let Err(e) = ws_write.send(dest_data).await {
            return;
        }
    }
}
pub async fn relay(
    Path((session_token, relay_info)): Path<(String, String)>,
    ws_upgrade: WebSocketUpgrade,
    State(server_state): State<Arc<ServerState>>,
) -> Response {
    debug!(
        session_token = { session_token.clone() },
        relay_info = { relay_info.clone() },
        "Receive websocket upgrade request."
    );
    ws_upgrade.on_upgrade(|ws| async move {
        let (auth_token, agent_encryption, proxy_encryption) = {
            let session_repository = server_state.session_repository();
            let session_repository = match session_repository.lock() {
                Ok(session_repository) => session_repository,
                Err(_) => {
                    error!(
                        session_token = { session_token.clone() },
                        "Fail to acquire session repository lock for session"
                    );
                    return;
                }
            };
            let Some(session) = session_repository.get(&session_token) else {
                return;
            };
            (
                session.auth_token().to_owned(),
                session.agent_encryption().clone(),
                session.proxy_encryption().clone(),
            )
        };
        let relay_info = match parse_relay_info(relay_info, &agent_encryption) {
            Ok(relay_info) => relay_info,
            Err(e) => {
                error!(
                    session_token = { session_token.clone() },
                    auth_token = { auth_token.clone() },
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
                    session_token = { session_token.clone() },
                    auth_token = { auth_token.clone() },
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
                        session_token = { session_token.clone() },
                        auth_token = { auth_token.clone() },
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
        tokio::spawn(relay_agent_to_dest(
            agent_encryption,
            ws_read,
            dest_transport_write,
        ));
        tokio::spawn(relay_dest_to_agent(
            proxy_encryption,
            ws_write,
            dest_transport_read,
        ));
    })
}
fn parse_relay_info(
    relay_info: String,
    agent_encryption: &Encryption,
) -> Result<RelayInfo, ProxyError> {
    let relay_info = hex::decode(relay_info)?;
    let relay_info = BASE64_STANDARD.decode(relay_info)?;
    let relay_info = match &agent_encryption {
        Encryption::Plain => relay_info,
        Encryption::Aes(aes_token) => decrypt_with_aes(aes_token, &relay_info)?,
    };
    let relay_info: RelayInfo = relay_info.try_into()?;
    Ok(relay_info)
}
