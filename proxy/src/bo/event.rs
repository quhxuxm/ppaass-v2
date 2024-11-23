use ppaass_domain::address::UnifiedAddress;
#[derive(Debug)]
pub enum ProxyServerEvent {
    ServerStartup,
    ServerStartFail,
    ServerTcpBind,
    AgentTcpConnected,
    TunnelInit(UnifiedAddress),
    SessionClosed(String),
}
