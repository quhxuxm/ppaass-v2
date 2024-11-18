use crate::bo::config::Config;
use crate::bo::event::AgentServerEvent;
use crate::crypto::AgentRsaCryptoFetcher;
use crate::error::AgentError;
use crate::handler::http::handle_http_client_tcp_stream;
use crate::handler::socks5::handle_socks5_client_tcp_stream;
use crate::handler::HandlerRequest;
use crate::session::AgentSession;
use crate::{publish_server_event, HttpClient};
use ppaass_crypto::random_32_bytes;
use ppaass_crypto::rsa::{RsaCrypto, RsaCryptoFetcher};
use ppaass_domain::relay::RelayUpgradeFailureReason;
use ppaass_domain::session::{CreateSessionRequest, CreateSessionResponse, Encryption};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::ops::Deref;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{channel, Receiver};
use tokio::sync::Mutex;
use tracing::error;
const SOCKS5_VERSION: u8 = 0x05;
const SOCKS4_VERSION: u8 = 0x04;
pub struct AgentServer {
    config: Arc<Config>,
    rsa_crypto_fetcher: Arc<AgentRsaCryptoFetcher>,
    session: Arc<Mutex<Option<AgentSession>>>,
}
impl AgentServer {
    pub fn new(config: Arc<Config>) -> Result<Self, AgentError> {
        Ok(Self {
            rsa_crypto_fetcher: Arc::new(AgentRsaCryptoFetcher::new(config.clone())?),
            config,
            session: Default::default(),
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
    async fn refresh_agent_session(
        config: Arc<Config>,
        rsa_crypto: &RsaCrypto,
        agent_encryption: Encryption,
        http_client: HttpClient,
        session: Arc<Mutex<Option<AgentSession>>>,
    ) -> Result<(), AgentError> {
        let create_session_request = match &agent_encryption {
            Encryption::Plain => CreateSessionRequest {
                agent_encryption: agent_encryption.clone(),
                auth_token: config.auth_token().to_owned(),
            },
            Encryption::Aes(aes_token) => {
                let encrypted_aes_token = rsa_crypto.encrypt(aes_token)?;
                CreateSessionRequest {
                    agent_encryption: Encryption::Aes(encrypted_aes_token.into()),
                    auth_token: config.auth_token().to_owned(),
                }
            }
        };
        let mut agent_session_lock = session.lock().await;
        let create_session_response = http_client
            .post(config.proxy_create_session_entry())
            .json(&create_session_request)
            .send()
            .await?;
        let CreateSessionResponse {
            proxy_encryption,
            session_token,
        } = create_session_response
            .json::<CreateSessionResponse>()
            .await?;
        let proxy_encryption = match proxy_encryption {
            Encryption::Plain => proxy_encryption,
            Encryption::Aes(rsa_encrypted_aes_token) => {
                Encryption::Aes(rsa_crypto.decrypt(&rsa_encrypted_aes_token)?.into())
            }
        };
        *agent_session_lock = Some(AgentSession {
            agent_encryption,
            proxy_encryption,
            session_token,
        });
        Ok(())
    }
    async fn concrete_start_server(
        config: Arc<Config>,
        rsa_crypto_fetcher: Arc<AgentRsaCryptoFetcher>,
        session: Arc<Mutex<Option<AgentSession>>>,
    ) -> Result<(), AgentError> {
        let rsa_crypto =
            rsa_crypto_fetcher
                .fetch(config.auth_token())?
                .ok_or(AgentError::RsaCryptoNotExist(
                    config.auth_token().to_owned(),
                ))?;
        let http_client = HttpClient::new();
        let agent_encryption = Encryption::Aes(random_32_bytes());
        Self::refresh_agent_session(
            config.clone(),
            &rsa_crypto,
            agent_encryption.clone(),
            http_client.clone(),
            session.clone(),
        )
            .await?;
        let tcp_listener = TcpListener::bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(0, 0, 0, 0),
            *config.port(),
        )))
            .await?;
        loop {
            let (client_tcp_stream, client_socket_addr) = tcp_listener.accept().await?;
            let http_client = http_client.clone();
            let config = config.clone();
            let (agent_encryption, proxy_encryption, session_token) = {
                let agent_session_lock =
                    session.lock().await;
                match agent_session_lock.deref() {
                    None => {
                        error!("Agent session is not initialized.");
                        continue;
                    }
                    Some(session) => (
                        session.agent_encryption.clone(),
                        session.proxy_encryption.clone(),
                        session.session_token.clone(),
                    ),
                }
            };
            let rsa_crypto = rsa_crypto.clone();
            let session = session.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_client_tcp_stream(
                    config.clone(),
                    HandlerRequest {
                        client_tcp_stream,
                        session_token,
                        proxy_encryption,
                        http_client: http_client.clone(),
                        client_socket_addr,
                        agent_encryption: agent_encryption.clone(),
                    },
                )
                    .await
                {
                    if let AgentError::RelayWebSocketUpgrade(
                        RelayUpgradeFailureReason::SessionNotFound,
                    ) = e
                    {
                        if let Err(e) = Self::refresh_agent_session(
                            config.clone(),
                            &rsa_crypto,
                            agent_encryption.clone(),
                            http_client.clone(),
                            session.clone(),
                        )
                            .await
                        {
                            error!("Fail to refresh agent session on previous expired: {e:?}");
                            return;
                        }
                    }
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
            let session = self.session.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    Self::concrete_start_server(config, rsa_crypto_fetcher, session).await
                {
                    error!("Fail to start agent server: {e:?}");
                    publish_server_event(server_event_tx, AgentServerEvent::ServerStartFail).await;
                }
            });
        }
        publish_server_event(server_event_tx, AgentServerEvent::ServerStartup).await;
        Ok(server_event_rx)
    }
}
