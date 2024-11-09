use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use clap::Parser;
use tokio::runtime::Builder;
use tracing::{error, info};
use agent::bo::command::CommandArgs;
use agent::bo::config::Config;
use agent::server::AgentServer;
const DEFAULT_CONFIG_FILE: &str = "config.toml";

pub fn main() -> Result<()> {
    let command = CommandArgs::parse();
    let config_file_path = command
        .config
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));
    let config_file_content = read_to_string(config_file_path)?;
    let config = Arc::new(toml::from_str::<Config>(&config_file_content)?);
    let runtime = Builder::new_multi_thread()
        .worker_threads(*config.worker_threads())
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let server = match AgentServer::new(config) {
            Ok(server) => server,
            Err(e) => {
                error!("Failed to build server object: {}", e);
                return;
            }
        };
        let mut server_event_rx = match server.start().await {
            Ok(server_event_rx) => server_event_rx,
            Err(e) => {
                error!("Failed to start server: {}", e);
                return;
            }
        };
        while let Some(server_event) = server_event_rx.recv().await {
            info!("Server event received: {:?}", server_event);
        }
    });
    Ok(())
}
