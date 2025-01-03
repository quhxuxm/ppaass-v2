use crate::address::UnifiedAddress;
use std::net::AddrParseError;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Unmatched unified address type: {0:?}")]
    UnmatchedUnifiedAddressType(UnifiedAddress),
    #[error(transparent)]
    ParseUnifiedAddressToIpAddress(#[from] AddrParseError),
    #[error("Failed to parse unified address to domain: {0:?}")]
    ParseUnifiedAddressToDomainAddress(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
