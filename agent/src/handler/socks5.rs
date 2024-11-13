use crate::bo::config::Config;
use crate::error::AgentError;
use crate::handler::HandlerRequest;
use crate::HttpClient;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use bytes::Bytes;
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
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::debug;
fn generate_relay_info_token(
    relay_info: RelayInfo,
    agent_encryption: &Encryption,
) -> Result<String, AgentError> {
    let encrypted_relay_info_bytes: Vec<u8> = match agent_encryption {
        Encryption::Plain => relay_info.try_into()?,
        Encryption::Aes(aes_token) => {
            let relay_info_bytes: Vec<u8> = relay_info.try_into()?;
            encrypt_with_aes(&aes_token, &relay_info_bytes)?.into()
        }
    };
    let encrypted_relay_info = BASE64_STANDARD.encode(&encrypted_relay_info_bytes);
    let encrypted_relay_info_bytes = encrypted_relay_info.as_bytes();
    Ok(hex::encode(&encrypted_relay_info_bytes))
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
        rsa_crypto,
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
            let (mut client_tcp_read, mut client_tcp_write) = client_tcp_stream.into_split();
            tokio::spawn(async move {
                loop {
                    let mut client_tcp_recv_buf = [0u8; 65536];
                    let client_data_size =
                        match client_tcp_read.read(&mut client_tcp_recv_buf).await {
                            Ok(client_data_size) => client_data_size,
                            Err(e) => {
                                return;
                            }
                        };
                    if client_data_size == 0 {
                        return;
                    }
                    let client_tcp_recv_buf = &client_tcp_recv_buf[..client_data_size];
                    let client_tcp_recv_buf = match &agent_encryption {
                        Encryption::Plain => client_tcp_recv_buf.to_vec(),
                        Encryption::Aes(aes_token) => match encrypt_with_aes(aes_token, client_tcp_recv_buf) {
                            Ok(client_data) => client_data,
                            Err(e) => {
                                return;
                            }
                        }
                    };
                    if let Err(e) = proxy_ws_write.send(Message::Binary(client_tcp_recv_buf)).await {
                        return;
                    };
                }
            });
            tokio::spawn(async move {
                loop {
                    let proxy_data =
                        match proxy_ws_read.next().await {
                            None => return,
                            Some(Err(e)) => {
                                return
                            }
                            Some(Ok(proxy_data)) => proxy_data,
                        };
                    let proxy_data = match proxy_data {
                        Message::Text(text_message) => {
                            debug!("Received text message from proxy: {text_message}" );
                            continue;
                        }
                        Message::Binary(proxy_data) => proxy_data,
                        Message::Ping(ping_data) => {
                            debug!("Received ping message from proxy:\n{}", pretty_hex::pretty_hex(&ping_data) );
                            continue;
                        }
                        Message::Pong(pong_data) => {
                            debug!("Received pong message from proxy:\n{}", pretty_hex::pretty_hex(&pong_data) );
                            continue;
                        }
                        Message::Close { code, reason } => {
                            debug!("Received close message from proxy with code: {code}, reason: {reason}");
                            return;
                        }
                    };
                    let proxy_data = match &proxy_encryption {
                        Encryption::Plain => {
                            proxy_data
                        }
                        Encryption::Aes(aes_token) => match decrypt_with_aes(aes_token, &proxy_data) {
                            Ok(proxy_data) => proxy_data,
                            Err(e) => {
                                return;
                            }
                        }
                    };
                    if let Err(e) = client_tcp_write.write_all(&proxy_data).await {
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
