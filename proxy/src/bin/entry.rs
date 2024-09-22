use anyhow::Result;
use clap::Parser;
use proxy::args::Args;
use proxy::config::Configuration;
use proxy::server::Server;
use std::sync::Arc;
use tokio::runtime::Builder;
use tracing::error;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::time::ChronoUtc;
fn init_tracing() -> Result<WorkerGuard> {
    let (tracing_non_blocking_appender, _tracing_appender_guard) =
        tracing_appender::non_blocking(std::io::stdout());
    let tracing_subscriber = tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .with_timer(ChronoUtc::default())
        .with_level(true)
        .with_max_level(LevelFilter::DEBUG)
        .with_writer(tracing_non_blocking_appender)
        .finish();
    tracing::subscriber::set_global_default(tracing_subscriber)?;
    Ok(_tracing_appender_guard)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let configuration_file_content = std::fs::read_to_string(args.configuration_path())?;
    let configuration = Arc::new(toml::de::from_str::<Configuration>(
        &configuration_file_content,
    )?);
    let _tracing_guard = init_tracing()?;
    let mut runtime_builder = Builder::new_multi_thread();
    let runtime = runtime_builder
        .worker_threads(configuration.worker_threads())
        .enable_all()
        .build()?;
    let server = Server::new(configuration);
    runtime.block_on(async move {
        if let Err(e) = server.run().await {
            error!(error=?e, "Fail to run proxy server.");
        }
    });
    Ok(())
}
