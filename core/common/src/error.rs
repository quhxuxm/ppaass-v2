use thiserror::Error;
use tracing::metadata::ParseLevelError;
#[derive(Debug, Error)]
pub enum CommonError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    ParseLogLevel(#[from] ParseLevelError),
}
