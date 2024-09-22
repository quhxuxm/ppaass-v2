use thiserror::Error;
#[derive(Error, Debug)]
pub enum ServerError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Fail to lock: {0}")]
    Lock(String),
    #[error("{0}")]
    Other(String),
}
