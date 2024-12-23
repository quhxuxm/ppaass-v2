use crate::bo::event::AgentServerEvent;
use tokio::sync::mpsc::Sender;
use tracing::error;
pub mod bo;
pub mod codec;
pub mod command;
pub mod config;
pub mod crypto;
mod error;
pub mod handler;
mod pool;
pub mod server;
pub async fn publish_server_event(
    server_event_tx: Sender<AgentServerEvent>,
    event: AgentServerEvent,
) {
    if let Err(e) = server_event_tx.send(event).await {
        error!("Failed to publish server event: {}", e);
    }
}
