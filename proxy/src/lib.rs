use crate::bo::event::ServerEvent;
use tokio::sync::mpsc::Sender;
use tracing::error;
pub mod bo;
mod crypto;
mod destination;
mod error;
mod handler;
pub mod server;
/// Publish the server event to console
pub async fn publish_server_event(server_event_tx: Sender<ServerEvent>, event: ServerEvent) {
    if let Err(e) = server_event_tx.send(event).await {
        error!("Failed to publish server event: {}", e);
    }
}
