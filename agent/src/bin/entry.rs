use anyhow::Result;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::LevelFilter;
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
#[tokio::main]
async fn main() -> Result<()> {
    let _tracing_worker_guard = init_tracing()?;
    Ok(())
}
