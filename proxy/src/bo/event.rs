#[derive(Debug)]
pub enum ProxyServerEvent {
    ServerStartup,
    ServerStartFail,
    ServerTcpBind,
    AgentTcpConnected,
}
