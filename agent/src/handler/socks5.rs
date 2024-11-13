use crate::bo::config::Config;
use crate::error::AgentError;
use crate::handler::HandlerRequest;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use ppaass_crypto::aes::{decrypt_with_aes, encrypt_with_aes};
use ppaass_domain::address::UnifiedAddress;
use ppaass_domain::relay::{RelayInfo, RelayInfoBuilder, RelayType};
use ppaass_domain::session::Encryption;
use reqwest_websocket::{Error, Message, RequestBuilderExt};
use socks5_impl::protocol::{
    handshake::Request as Socks5HandshakeRequest, handshake::Response as Socks5HandshakeResponse,
    Address, AsyncStreamOperation, AuthMethod, Command, Reply, Request as Socks5Request, Response,
};
use std::sync::Arc;
use tokio_util::codec::{BytesCodec, Framed};
use tracing::{debug, error};
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
pub async fn handle_socks5_client_tcp_stream(
    config: Arc<Config>,
    request: HandlerRequest,
) -> Result<(), AgentError> {
    let HandlerRequest {
        mut client_tcp_stream,
        session_token,
        proxy_encryption,
        http_client,
        client_socket_addr,
        agent_encryption,
    } = request;
    let auth_request =
        Socks5HandshakeRequest::retrieve_from_async_stream(&mut client_tcp_stream).await?;
    debug!("Receive client socks5 handshake auth request: {auth_request:?}");
    let auth_response = Socks5HandshakeResponse::new(AuthMethod::NoAuth);
    auth_response
        .write_to_async_stream(&mut client_tcp_stream)
        .await?;
    let init_request = Socks5Request::retrieve_from_async_stream(&mut client_tcp_stream).await?;
    debug!("Receive client socks5 handshake init request: {init_request:?}");
    match init_request.command {
        Command::Connect => {
            let relay_info = match &init_request.address {
                Address::SocketAddress(socket_addr) => {
                    let mut relay_info_builder = RelayInfoBuilder::default();
                    relay_info_builder
                        .dst_address((*socket_addr).into())
                        .src_address(client_socket_addr.into())
                        .relay_type(RelayType::Tcp);
                    relay_info_builder.build()?
                }
                Address::DomainAddress(domain, port) => {
                    let mut relay_info_builder = RelayInfoBuilder::default();
                    relay_info_builder
                        .dst_address(UnifiedAddress::Domain {
                            host: domain.to_owned(),
                            port: *port,
                        })
                        .src_address(client_socket_addr.into())
                        .relay_type(RelayType::Tcp);
                    relay_info_builder.build()?
                }
            };
            let relay_info_token =
                generate_relay_info_token(relay_info.clone(), &agent_encryption)?;
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
            let init_response = Response::new(Reply::Succeeded, init_request.address);
            init_response
                .write_to_async_stream(&mut client_tcp_stream)
                .await?;
            let (mut proxy_ws_write, mut proxy_ws_read) = relay_ws.split();
            let client_framed = Framed::with_capacity(client_tcp_stream, BytesCodec::new(), 1024 * 1024 * 64);
            let (mut client_tcp_write, mut client_tcp_read) = client_framed.split::<BytesMut>();
            {
                let session_token = session_token.clone();
                let relay_info_token = relay_info_token.clone();
                tokio::spawn(async move {
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
                        let client_data = match &agent_encryption {
                            Encryption::Plain => client_data.to_vec(),
                            Encryption::Aes(aes_token) => {
                                match encrypt_with_aes(aes_token, &client_data) {
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
                                }
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
                            debug!(session_token={&session_token}, relay_info={&relay_info_token},"Received close message from proxy with code: {code}, reason: {reason}");
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
                        Encryption::Aes(aes_token) => {
                            match decrypt_with_aes(aes_token, &proxy_data) {
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
                            }
                        }
                    };
                    if let Err(e) = client_tcp_write.send(BytesMut::from_iter(&proxy_data)).await {
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
        Command::Bind => {}
        Command::UdpAssociate => {}
    }
    Ok(())
}
