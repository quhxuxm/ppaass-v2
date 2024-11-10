use crate::bo::event::AgentServerEvent;
use tokio::sync::mpsc::Sender;
use tracing::error;
pub mod bo;
pub mod crypto;
mod error;
pub mod handler;
pub mod server;

pub type HttpClient = reqwest::Client;
pub async fn publish_server_event(
    server_event_tx: Sender<AgentServerEvent>,
    event: AgentServerEvent,
) {
    if let Err(e) = server_event_tx.send(event).await {
        error!("Failed to publish server event: {}", e);
    }
}
