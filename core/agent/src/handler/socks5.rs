use crate::bo::state::ServerState;
use crate::error::AgentError;
use crate::handler::{relay, tunnel_init, RelayRequest, TunnelInitHandlerResponse};
use ppaass_domain::address::UnifiedAddress;
use socks5_impl::protocol::{
    handshake::Request as Socks5HandshakeRequest, handshake::Response as Socks5HandshakeResponse,
    Address, AsyncStreamOperation, AuthMethod, Command, Reply, Request as Socks5Request, Response,
};
use tokio::net::TcpStream;
use tracing::debug;
pub async fn handle_socks5_client_tcp_stream(
    mut client_tcp_stream: TcpStream,
    server_state: ServerState,
) -> Result<(), AgentError> {
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
            debug!("Receive socks5 CONNECT command: {client_tcp_stream:?}");
            let TunnelInitHandlerResponse {
                proxy_tcp_stream,
                agent_encryption,
                proxy_encryption,
                destination_address,
            } = tunnel_init(
                match &init_request.address {
                    Address::SocketAddress(dst_addr) => dst_addr.into(),
                    Address::DomainAddress(host, port) => UnifiedAddress::Domain {
                        host: host.clone(),
                        port: *port,
                    },
                },
                server_state.clone(),
            )
            .await?;
            let init_response = Response::new(Reply::Succeeded, init_request.address);
            init_response
                .write_to_async_stream(&mut client_tcp_stream)
                .await?;
            relay(
                RelayRequest {
                    client_tcp_stream,
                    proxy_tcp_stream,
                    agent_encryption,
                    proxy_encryption,
                    init_data: None,
                    destination_address,
                },
                server_state,
            )
            .await?;
        }
        Command::Bind => {
            debug!("Receive socks5 BIND command: {client_tcp_stream:?}");
            return Err(AgentError::UnsupportedSocksV5Command("BIND".to_string()));
        }
        Command::UdpAssociate => {
            debug!("Receive socks5 UDP ASSOCIATE command: {client_tcp_stream:?}");
            return Err(AgentError::UnsupportedSocksV5Command(
                "UDP_ASSOCIATE".to_string(),
            ));
        }
    }
    Ok(())
}
