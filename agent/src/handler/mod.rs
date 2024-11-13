use crate::error::AgentError;
use crate::HttpClient;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use ppaass_crypto::aes::encrypt_with_aes;
use ppaass_domain::relay::RelayInfo;
use ppaass_domain::session::Encryption;
use std::net::SocketAddr;
use tokio::net::TcpStream;
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