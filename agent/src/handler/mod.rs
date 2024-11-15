use crate::bo::config::Config;
use crate::error::AgentError;
use crate::HttpClient;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use bytes::{Bytes, BytesMut};
use futures_util::{SinkExt, StreamExt};
use ppaass_crypto::aes::{decrypt_with_aes, encrypt_with_aes};
use ppaass_domain::relay::RelayInfo;
use ppaass_domain::session::Encryption;
use reqwest_websocket::{Message, RequestBuilderExt, WebSocket};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_util::codec::{BytesCodec, Framed};
use tracing::{debug, error};
pub mod http;
pub mod socks5;
pub struct HandlerRequest {
    pub client_tcp_stream: TcpStream,
    pub session_token: String,
    pub agent_encryption: Encryption,
    pub proxy_encryption: Encryption,
    pub http_client: HttpClient,
    pub client_socket_addr: SocketAddr,
}
fn generate_relay_info_token(
    relay_info: RelayInfo,
    agent_encryption: &Encryption,
) -> Result<String, AgentError> {
    let encrypted_relay_info_bytes: Vec<u8> = match agent_encryption {
        Encryption::Plain => relay_info.try_into()?,
        Encryption::Aes(aes_token) => {
            let relay_info_bytes: Vec<u8> = relay_info.try_into()?;
            encrypt_with_aes(aes_token, &relay_info_bytes)?
        }
    };
    let encrypted_relay_info = BASE64_STANDARD.encode(&encrypted_relay_info_bytes);
    let encrypted_relay_info_bytes = encrypted_relay_info.as_bytes();
    Ok(hex::encode(encrypted_relay_info_bytes).to_uppercase())
}
async fn generate_relay_websocket(
    session_token: &str,
    relay_info: RelayInfo,
    agent_encryption: &Encryption,
    config: &Config,
    http_client: &HttpClient,
) -> Result<(WebSocket, String), AgentError> {
    let relay_info_token = generate_relay_info_token(relay_info.clone(), agent_encryption)?;
    let relay_url = format!(
        "{}/{}/{}",
        config.proxy_relay_entry(),
        session_token,
        relay_info_token
    );
    debug!("Begin to create relay websocket on proxy (GET): {relay_url}");
    let relay_upgrade_connection = http_client.get(&relay_url).upgrade().send().await?;
    debug!("Upgrade relay connection to websocket on proxy (UPGRADE): {relay_url}");
    let relay_ws = relay_upgrade_connection.into_websocket().await?;
    debug!("Create relay connection websocket on proxy success: {relay_url}");
    Ok((relay_ws, relay_info_token))
}

fn encrypt_agent_data(data: Bytes, agent_encryption: &Encryption) -> Result<Vec<u8>, AgentError> {
    match &agent_encryption {
        Encryption::Plain => Ok(data.to_vec()),
        Encryption::Aes(aes_token) => Ok(encrypt_with_aes(aes_token, &data)?),
    }
}

struct RelayProxyDataRequest {
    client_tcp_stream: TcpStream,
    proxy_websocket: WebSocket,
    session_token: String,
    agent_encryption: Encryption,
    proxy_encryption: Encryption,
    relay_info_token: String,
    initial_data: Option<Bytes>,
}
async fn relay_proxy_data(
    RelayProxyDataRequest {
        client_tcp_stream,
        proxy_websocket,
        session_token,
        agent_encryption,
        proxy_encryption,
        relay_info_token,
        initial_data,
    }: RelayProxyDataRequest,
) {
    let (mut proxy_ws_write, mut proxy_ws_read) = proxy_websocket.split();
    let client_framed =
        Framed::with_capacity(client_tcp_stream, BytesCodec::new(), 1024 * 1024 * 64);
    let (mut client_tcp_write, mut client_tcp_read) = client_framed.split::<BytesMut>();
    {
        let session_token = session_token.clone();
        let relay_info_token = relay_info_token.clone();
        tokio::spawn(async move {
            if let Some(initial_data) = initial_data {
                let initial_data = match encrypt_agent_data(initial_data, &agent_encryption) {
                    Ok(initial_data) => initial_data,
                    Err(e) => {
                        error!(
                            session_token = { &session_token },
                            relay_info = { &relay_info_token },
                            "Fail to aes encrypt client data: {e:?}"
                        );
                        if let Err(e) = proxy_ws_write.close().await {
                            error!(session_token={session_token}, relay_info={relay_info_token},"Fail to close proxy websocket connection on aes encrypt client data fail: {e:?}");
                        };
                        return;
                    }
                };
                if let Err(e) = proxy_ws_write.send(Message::Binary(initial_data)).await {
                    error!(
                        session_token = { &session_token },
                        relay_info = { &relay_info_token },
                        "Fail write client data to proxy: {e:?}"
                    );
                    if let Err(e) = proxy_ws_write.close().await {
                        error!(session_token={session_token}, relay_info={relay_info_token},"Fail to close proxy websocket connection on write client data to proxy fail: {e:?}");
                    };
                    return;
                };
            }

            loop {
                let client_data = match client_tcp_read.next().await {
                    None => {
                        debug!(
                            session_token = { &session_token },
                            relay_info = { &relay_info_token },
                            "Client data exhausted"
                        );
                        if let Err(e) = proxy_ws_write.close().await {
                            error!(session_token={session_token}, relay_info={relay_info_token},"Fail to close proxy websocket connection on client data exhausted: {e:?}");
                        };
                        return;
                    }
                    Some(Ok(client_data)) => client_data,
                    Some(Err(e)) => {
                        error!(
                            session_token = { &session_token },
                            relay_info = { &relay_info_token },
                            "Fail to read client data: {e:?}"
                        );
                        if let Err(e) = proxy_ws_write.close().await {
                            error!(session_token={session_token}, relay_info={relay_info_token},"Fail to close proxy websocket connection on read client data have error: {e:?}");
                        };
                        return;
                    }
                };
                let client_data = match encrypt_agent_data(client_data.freeze(), &agent_encryption)
                {
                    Ok(client_data) => client_data,
                    Err(e) => {
                        error!(
                            session_token = { &session_token },
                            relay_info = { &relay_info_token },
                            "Fail to aes encrypt client data: {e:?}"
                        );
                        if let Err(e) = proxy_ws_write.close().await {
                            error!(session_token={session_token}, relay_info={relay_info_token},"Fail to close proxy websocket connection on aes encrypt client data fail: {e:?}");
                        };
                        return;
                    }
                };
                if let Err(e) = proxy_ws_write.send(Message::Binary(client_data)).await {
                    error!(
                        session_token = { &session_token },
                        relay_info = { &relay_info_token },
                        "Fail write client data to proxy: {e:?}"
                    );
                    if let Err(e) = proxy_ws_write.close().await {
                        error!(session_token={session_token}, relay_info={relay_info_token},"Fail to close proxy websocket connection on write client data to proxy fail: {e:?}");
                    };
                    return;
                };
            }
        });
    }
    tokio::spawn(async move {
        loop {
            let proxy_data = match proxy_ws_read.next().await {
                None => {
                    if let Err(e) = client_tcp_write.close().await {
                        error!(
                            session_token = { &session_token },
                            relay_info = { &relay_info_token },
                            "Fail to close client tcp connection when proxy exhausted: {e:?}"
                        );
                    }
                    return;
                }
                Some(Err(e)) => {
                    error!(
                        session_token = { &session_token },
                        relay_info = { &relay_info_token },
                        "Fail read data from proxy: {e:?}"
                    );
                    if let Err(e) = client_tcp_write.close().await {
                        error!(
                            session_token = { &session_token },
                            relay_info = { &relay_info_token },
                            "Fail to close client tcp connection when read proxy fail: {e:?}"
                        );
                    }
                    return;
                }
                Some(Ok(proxy_data)) => proxy_data,
            };
            let proxy_data = match proxy_data {
                Message::Text(text_message) => {
                    debug!(
                        session_token = { &session_token },
                        relay_info = { &relay_info_token },
                        "Received text message from proxy: {text_message}"
                    );
                    continue;
                }
                Message::Binary(proxy_data) => proxy_data,
                Message::Ping(ping_data) => {
                    debug!(
                        session_token = { &session_token },
                        relay_info = { &relay_info_token },
                        "Received ping message from proxy:\n{}",
                        pretty_hex::pretty_hex(&ping_data)
                    );
                    continue;
                }
                Message::Pong(pong_data) => {
                    debug!(
                        session_token = { &session_token },
                        relay_info = { &relay_info_token },
                        "Received pong message from proxy:\n{}",
                        pretty_hex::pretty_hex(&pong_data)
                    );
                    continue;
                }
                Message::Close { code, reason } => {
                    debug!(
                        session_token = { &session_token },
                        relay_info = { &relay_info_token },
                        "Received close message from proxy with code: {code}, reason: {reason}"
                    );
                    if let Err(e) = client_tcp_write.close().await {
                        error!(
                            session_token = { &session_token },
                            relay_info = { &relay_info_token },
                            "Fail to close client tcp connection when proxy websocket close: {e:?}"
                        );
                    }
                    return;
                }
            };
            let proxy_data = match &proxy_encryption {
                Encryption::Plain => proxy_data,
                Encryption::Aes(aes_token) => match decrypt_with_aes(aes_token, &proxy_data) {
                    Ok(proxy_data) => proxy_data,
                    Err(e) => {
                        error!(
                            session_token = { &session_token },
                            relay_info = { &relay_info_token },
                            "Fail read decrypt aes data from proxy: {e:?}"
                        );
                        if let Err(e) = client_tcp_write.close().await {
                            error!(
                                            session_token = { &session_token },
                                            relay_info = { &relay_info_token },
                                            "Fail to close client tcp connection when decrypt proxy data with aes fail: {e:?}"
                                        );
                        }
                        return;
                    }
                },
            };
            if let Err(e) = client_tcp_write
                .send(BytesMut::from_iter(&proxy_data))
                .await
            {
                error!(
                    session_token = { &session_token },
                    relay_info = { &relay_info_token },
                    "Fail to write proxy data to client: {e:?}"
                );
                if let Err(e) = client_tcp_write.close().await {
                    error!(
                        session_token = { &session_token },
                        relay_info = { &relay_info_token },
                        "Fail to close client tcp connection when send data to client fail: {e:?}"
                    );
                }
                return;
            };
        }
    });
}
