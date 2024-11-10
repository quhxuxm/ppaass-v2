use crate::HttpClient;
use ppaass_crypto::rsa::RsaCrypto;
use ppaass_domain::session::Encryption;
use std::net::SocketAddr;
use std::sync::Arc;
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
    pub rsa_crypto: Arc<RsaCrypto>,
}
