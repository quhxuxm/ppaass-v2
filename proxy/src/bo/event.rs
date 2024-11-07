#[derive(Debug)]
pub enum ServerEvent {
    ServerStartup,
    ServerStartFail,
    ServerTcpBind,
    AgentTcpConnected,
}
