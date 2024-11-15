use crate::bo::config::Config;
use crate::error::AgentError;
use crate::HttpClient;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use ppaass_crypto::aes::encrypt_with_aes;
use ppaass_domain::relay::RelayInfo;
use ppaass_domain::session::Encryption;
use reqwest_websocket::{RequestBuilderExt, WebSocket};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing::debug;
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
async fn generate_relay_websocket(session_token: &str, relay_info: RelayInfo, agent_encryption: &Encryption, config: &Config, http_client: &HttpClient) -> Result<(WebSocket, String), AgentError> {
    let relay_info_token =
        generate_relay_info_token(relay_info.clone(), agent_encryption)?;
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