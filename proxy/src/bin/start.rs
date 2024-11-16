use anyhow::Result;
use clap::Parser;
use proxy::bo::command::CommandArgs;
use proxy::bo::config::Config;
use proxy::server::ProxyServer;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::runtime::Builder;
use tracing::{error, info, Level};
const DEFAULT_CONFIG_FILE: &str = "config.toml";
pub fn main() -> Result<()> {
    let command = CommandArgs::parse();
    let config_file_path = command
        .config
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));
    let config_file_content = read_to_string(config_file_path)?;
    let config = Arc::new(toml::from_str::<Config>(&config_file_content)?);
    tracing_subscriber::fmt()
        .with_max_level(Level::from_str(config.max_log_level())?)
        .init();
    let runtime = Builder::new_multi_thread()
        .worker_threads(*config.worker_threads())
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let (server, server_event_rx) = match ProxyServer::new(config) {
            Ok(server) => server,
            Err(e) => {
                error!("Failed to build server object: {}", e);
                return;
            }
        };
        let mut server_event_rx = match server.start(server_event_rx).await {
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
