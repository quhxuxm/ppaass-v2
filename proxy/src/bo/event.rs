#[derive(Debug)]
pub enum ProxyServerEvent {
    ServerStartup,
    ServerStartFail,
    ServerTcpBind,
    AgentTcpConnected,
    SessionStarted(String),
    SessionClosed(String),
}
