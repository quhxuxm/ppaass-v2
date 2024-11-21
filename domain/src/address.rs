use crate::error::DomainError;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use std::net::{SocketAddr, ToSocketAddrs};
/// The unified address which can support both IP V4, IP V6 and Domain
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum UnifiedAddress {
    Domain { host: String, port: u16 },
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
                2 => {
                    let domain = domain_parts[0];
                    let port = domain_parts[1].parse::<u16>().map_err(|_| {
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
impl TryFrom<&UnifiedAddress> for Vec<SocketAddr> {
    type Error = DomainError;
    fn try_from(value: &UnifiedAddress) -> Result<Self, Self::Error> {
        match value {
            UnifiedAddress::Domain { host, port } => {
                let socket_addresses = format!("{host}:{port}").to_socket_addrs()?;
                let socket_addresses = socket_addresses.collect::<Vec<SocketAddr>>();
                Ok(socket_addresses)
            }
            UnifiedAddress::Ip(socket_addr) => Ok(vec![*socket_addr]),
        }
    }
}
impl From<SocketAddr> for UnifiedAddress {
    fn from(value: SocketAddr) -> Self {
        UnifiedAddress::Ip(value)
    }
}
