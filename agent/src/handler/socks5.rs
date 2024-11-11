use crate::bo::config::Config;
use crate::error::AgentError;
use crate::handler::HandlerRequest;
use crate::HttpClient;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use bytes::Bytes;
use ppaass_crypto::aes::encrypt_with_aes;
use ppaass_domain::address::UnifiedAddress;
use ppaass_domain::relay::{RelayInfo, RelayInfoBuilder, RelayType};
use ppaass_domain::session::Encryption;
use reqwest_websocket::RequestBuilderExt;
use socks5_impl::protocol::{
    handshake::Request as Socks5HandshakeRequest, handshake::Response as Socks5HandshakeResponse,
    Address, AsyncStreamOperation, AuthMethod, Command, Request as Socks5Request,
};
use std::sync::Arc;
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
            let relay_info = match init_request.address {
                Address::SocketAddress(socket_addr) => {
                    let mut relay_info_builder = RelayInfoBuilder::default();
                    relay_info_builder
                        .dst_address(socket_addr.into())
                        .src_address(client_socket_addr.into())
                        .relay_type(RelayType::Tcp);
                    relay_info_builder.build()?
                }
                Address::DomainAddress(domain, port) => {
                    let mut relay_info_builder = RelayInfoBuilder::default();
                    relay_info_builder
                        .dst_address(UnifiedAddress::Domain { host: domain, port })
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
            let relay_url = format!("{}", config.proxy_relay_entry(),);
            debug!("Begin to create relay websocket on proxy (GET): {relay_url}");
            let relay_upgrade_connection = http_client.get(&relay_url).upgrade().send().await?;
            debug!("Upgrade relay connection to websocket on proxy (UPGRADE): {relay_url}");
            let relay_websocket = relay_upgrade_connection.into_websocket().await?;
            debug!("Create relay connection websocket on proxy success: {relay_url}");
        }
        Command::Bind => {}
        Command::UdpAssociate => {}
    }
    Ok(())
}
