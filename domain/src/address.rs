use crate::error::DomainError;
use bytes::Bytes;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use std::net::SocketAddr;
/// The unified address which can support both IP V4, IP V6 and Domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnifiedAddress {
    Domain { host: String, port: u32 },
    Ip(SocketAddr),
}

impl Display for UnifiedAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UnifiedAddress::Domain { host, port } => write!(f, "{}:{}", host, port),
            UnifiedAddress::Ip(socket_addr) => match socket_addr {
                SocketAddr::V4(ip_v4_addr) => {
                    write!(f, "{}:{}", ip_v4_addr.ip(), socket_addr.port())
                }
                SocketAddr::V6(ip_v6_addr) => {
                    write!(f, "{}:{}", ip_v6_addr.ip(), socket_addr.port())
                }
            },
        }
    }
}

impl TryFrom<String> for UnifiedAddress {
    type Error = DomainError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if let Ok(ip_address) = value.parse::<SocketAddr>() {
            Ok(Self::Ip(ip_address))
        } else {
            let domain_parts = value.split(":").collect::<Vec<&str>>();
            match domain_parts.len() {
                parts_num if parts_num > 2 => {
                    Err(DomainError::ParseUnifiedAddressToDomainAddress(value))
                }
                parts_num if parts_num == 2 => {
                    let domain = domain_parts[0];
                    let port = domain_parts[1].parse::<u32>().map_err(|_| {
                        DomainError::ParseUnifiedAddressToDomainAddress(value.clone())
                    })?;
                    Ok(Self::Domain {
                        host: domain.to_string(),
                        port,
                    })
                }
                _ => {
                    let domain = domain_parts[0];
                    Ok(Self::Domain {
                        host: domain.to_string(),
                        port: 80,
                    })
                }
            }
        }
    }
}

impl TryFrom<Bytes> for UnifiedAddress {
    type Error = DomainError;
    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        let result = bincode::deserialize::<UnifiedAddress>(&value)?;
        Ok(result)
    }
}

impl TryFrom<UnifiedAddress> for Bytes {
    type Error = DomainError;
    fn try_from(value: UnifiedAddress) -> Result<Self, Self::Error> {
        let result = bincode::serialize(&value)?;
        Ok(result.into())
    }
}

impl TryFrom<UnifiedAddress> for SocketAddr {
    type Error = DomainError;
    fn try_from(value: UnifiedAddress) -> Result<Self, Self::Error> {
        match value {
            UnifiedAddress::Domain { .. } => {
                Err(DomainError::UnmatchedUnifiedAddressType(value.clone()))
            }
            UnifiedAddress::Ip(socket_addr) => Ok(socket_addr),
        }
    }
}
