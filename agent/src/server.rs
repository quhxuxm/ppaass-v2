use crate::bo::config::Config;
use crate::bo::event::AgentServerEvent;
use crate::crypto::AgentRsaCryptoFetcher;
use crate::error::AgentError;
use crate::handler::http::handle_http_client_tcp_stream;
use crate::handler::socks5::handle_socks5_client_tcp_stream;
use crate::handler::HandlerRequest;
use crate::{publish_server_event, HttpClient};
use bytes::Bytes;
use futures_util::SinkExt;
use ppaass_crypto::random_32_bytes;
use ppaass_crypto::rsa::{RsaCrypto, RsaCryptoFetcher};
use ppaass_domain::session::{CreateSessionRequestBuilder, CreateSessionResponse, Encryption};
use reqwest_websocket::{Message, RequestBuilderExt};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{channel, Receiver};
use tracing::{debug, error};
const SOCKS5_VERSION: u8 = 0x05;
const SOCKS4_VERSION: u8 = 0x04;
pub struct AgentServer {
    config: Arc<Config>,
    rsa_crypto_fetcher: Arc<AgentRsaCryptoFetcher>,
}
impl AgentServer {
    pub fn new(config: Arc<Config>) -> Result<Self, AgentError> {
        Ok(Self {
            rsa_crypto_fetcher: Arc::new(AgentRsaCryptoFetcher::new(config.clone())?),
            config,
        })
    }
    async fn switch_protocol(client_tcp_stream: &TcpStream) -> Result<u8, AgentError> {
        let mut protocol = [0u8; 1];
        client_tcp_stream.peek(&mut protocol).await?;
        if protocol.is_empty() {
            Err(AgentError::ClientTcpConnectionExhausted)
        } else {
            Ok(protocol[0])
        }
    }
    async fn handle_client_tcp_stream(
        config: Arc<Config>,
        request: HandlerRequest,
    ) -> Result<(), AgentError> {
        let protocol = Self::switch_protocol(&request.client_tcp_stream).await?;
        match protocol {
            SOCKS5_VERSION => handle_socks5_client_tcp_stream(config, request).await,
            SOCKS4_VERSION => Err(AgentError::UnsupportedSocksV4Protocol),
            _ => handle_http_client_tcp_stream(config, request).await,
        }
    }
    async fn create_session(
        config: Arc<Config>,
        rsa_crypto: &RsaCrypto,
        agent_aes_token: Bytes,
        http_client: HttpClient,
    ) -> Result<CreateSessionResponse, AgentError> {
        let encrypted_aes_token = rsa_crypto.encrypt(&agent_aes_token)?;
        let mut create_session_request_builder = CreateSessionRequestBuilder::default();
        create_session_request_builder.auth_token(config.auth_token().to_owned());
        create_session_request_builder
            .agent_encryption(Encryption::Aes(encrypted_aes_token.into()));
        let create_session_request = create_session_request_builder.build()?;
        let create_session_response = http_client
            .post(config.proxy_create_session_entry())
            .json(&create_session_request)
            .send()
            .await?;
        let create_session_response = create_session_response
            .json::<CreateSessionResponse>()
            .await?;
        Ok(create_session_response)
    }
    async fn concrete_start_server(
        config: Arc<Config>,
        rsa_crypto_fetcher: Arc<AgentRsaCryptoFetcher>,
    ) -> Result<(), AgentError> {
        let rsa_crypto =
            rsa_crypto_fetcher
                .fetch(config.auth_token())?
                .ok_or(AgentError::RsaCryptoNotExist(
                    config.auth_token().to_owned(),
                ))?;
        let http_client = HttpClient::new();
        let agent_aes_token = random_32_bytes();
        let create_session_response = Self::create_session(
            config.clone(),
            &rsa_crypto,
            agent_aes_token.clone(),
            http_client.clone(),
        )
        .await?;

        let tcp_listener = TcpListener::bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(0, 0, 0, 0),
            *config.port(),
        )))
        .await?;
        loop {
            let session_token = create_session_response.session_token().to_owned();
            let proxy_encryption = create_session_response.proxy_encryption().clone();
            let (client_tcp_stream, client_socket_addr) = tcp_listener.accept().await?;
            let http_client = http_client.clone();
            let rsa_crypto = rsa_crypto.clone();
            let agent_aes_token = agent_aes_token.clone();
            let config = config.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_client_tcp_stream(
                    config,
                    HandlerRequest {
                        client_tcp_stream,
                        session_token,
                        proxy_encryption,
                        http_client,
                        client_socket_addr,
                        rsa_crypto,
                        agent_encryption: Encryption::Aes(agent_aes_token),
                    },
                )
                .await
                {
                    error!("Fail to handle client tcp stream [{client_socket_addr:?}]: {e:?}")
                }
            });
        }
    }
    pub async fn start(self) -> Result<Receiver<AgentServerEvent>, AgentError> {
        let (server_event_tx, server_event_rx) = channel::<AgentServerEvent>(1024);
        {
            let server_event_tx = server_event_tx.clone();
            let config = self.config.clone();
            let rsa_crypto_fetcher = self.rsa_crypto_fetcher.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::concrete_start_server(config, rsa_crypto_fetcher).await {
                    error!("Fail to start agent server: {e:?}");
                    publish_server_event(server_event_tx, AgentServerEvent::ServerStartFail).await;
                }
            });
        }
        publish_server_event(server_event_tx, AgentServerEvent::ServerStartup).await;
        Ok(server_event_rx)
    }
}
