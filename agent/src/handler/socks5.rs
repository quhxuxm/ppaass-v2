use crate::bo::config::Config;
use crate::error::AgentError;
use crate::handler::{
    generate_relay_websocket, relay_proxy_data, HandlerRequest, RelayProxyDataRequest,
};
use ppaass_domain::address::UnifiedAddress;
use ppaass_domain::relay::{RelayInfo, RelayType};
use socks5_impl::protocol::{
    handshake::Request as Socks5HandshakeRequest, handshake::Response as Socks5HandshakeResponse,
    Address, AsyncStreamOperation, AuthMethod, Command, Reply, Request as Socks5Request, Response,
};
use std::sync::Arc;
use tracing::debug;
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
                Address::SocketAddress(socket_addr) => RelayInfo {
                    dst_address: (*socket_addr).into(),
                    src_address: client_socket_addr.into(),
                    relay_type: RelayType::Tcp,
                },
                Address::DomainAddress(domain, port) => RelayInfo {
                    dst_address: UnifiedAddress::Domain {
                        host: domain.to_owned(),
                        port: *port,
                    },
                    src_address: client_socket_addr.into(),
                    relay_type: RelayType::Tcp,
                },
            };
            let (proxy_websocket, relay_info_token) = generate_relay_websocket(
                &session_token,
                relay_info,
                &agent_encryption,
                &config,
                &http_client,
            )
            .await?;
            let init_response = Response::new(Reply::Succeeded, init_request.address);
            init_response
                .write_to_async_stream(&mut client_tcp_stream)
                .await?;
            relay_proxy_data(
                &config,
                RelayProxyDataRequest {
                    client_tcp_stream,
                    proxy_websocket,
                    session_token,
                    agent_encryption,
                    proxy_encryption,
                    relay_info_token,
                    initial_data: None,
                },
            )
            .await;
        }
        Command::Bind => {}
        Command::UdpAssociate => {}
    }
    Ok(())
}
