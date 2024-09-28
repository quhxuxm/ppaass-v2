use thiserror::Error;
#[derive(Error, Debug)]
pub enum DomainError {
    #[error(transparent)]
    BincodeError(#[from] bincode::Error),
}
