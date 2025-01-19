use crate::error::CommonError;
use std::path::Path;
use std::str::FromStr;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::time::ChronoUtc;
pub mod error;

/// Init the logger
pub fn init_logger(
    // The folder to store the log file
    log_folder: &Path,
    // The log name prefix
    log_name_prefix: &str,
    // The max log level
    max_log_level: &str,
) -> Result<WorkerGuard, CommonError> {
    let (trace_file_appender, _trace_appender_guard) = tracing_appender::non_blocking(
        tracing_appender::rolling::daily(log_folder, log_name_prefix),
    );
    tracing_subscriber::fmt()
        .with_max_level(Level::from_str(max_log_level)?)
        .with_writer(trace_file_appender)
        .with_line_number(true)
        .with_level(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_timer(ChronoUtc::rfc_3339())
        .with_ansi(false)
        .init();
    Ok(_trace_appender_guard)
}
