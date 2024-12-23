use crate::bo::event::ProxyServerEvent;
use tokio::sync::mpsc::Sender;
use tracing::error;
pub mod bo;
mod codec;
pub mod command;
pub mod config;
mod crypto;
mod destination;
mod error;
mod handler;
pub mod server;
/// Publish the server event to console
pub async fn publish_server_event(
    server_event_tx: &Sender<ProxyServerEvent>,
    event: ProxyServerEvent,
) {
    if let Err(e) = server_event_tx.send(event).await {
        error!("Failed to publish server event: {}", e);
    }
}
