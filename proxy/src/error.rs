use thiserror::Error;
#[derive(Debug, Error)]
pub enum ServerError{
    #[error(transparent)]
    Io(#[from] std::io::Error)
}