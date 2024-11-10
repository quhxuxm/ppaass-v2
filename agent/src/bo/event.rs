#[derive(Debug)]
pub enum AgentServerEvent {
    ServerStartup,
    ServerStartFail,
    ServerTcpBind,
    AgentTcpConnected,
}
