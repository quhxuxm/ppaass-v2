use bytes::Bytes;
use derive_more::Constructor;
use std::net::SocketAddr;
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UnifyAddress {
    Ip(SocketAddr),
    Domain { host: String, port: u16 },
}

#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub enum TunnelType {
    Tcp,
    Udp,
}

#[derive(Debug, Constructor)]
pub struct TunnelInit {
    src_address: UnifyAddress,
    dest_address: UnifyAddress,
    tunnel_type: TunnelType,
}

impl TunnelInit {
    pub fn src_address(&self) -> &UnifyAddress {
        &self.src_address
    }

    pub fn dest_address(&self) -> &UnifyAddress {
        &self.dest_address
    }

    pub fn tunnel_type(&self) -> TunnelType {
        self.tunnel_type
    }
}

#[derive(Debug, Constructor)]
pub struct TunnelData {
    src_address: UnifyAddress,
    dest_address: UnifyAddress,
    tunnel_type: TunnelType,
    data: Bytes,
}

impl TunnelData {
    pub fn src_address(&self) -> &UnifyAddress {
        &self.src_address
    }
    pub fn dest_address(&self) -> &UnifyAddress {
        &self.dest_address
    }
    pub fn tunnel_type(&self) -> TunnelType {
        self.tunnel_type
    }
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Debug, Constructor)]
pub struct TunnelClose {
    src_address: UnifyAddress,
    dest_address: UnifyAddress,
    tunnel_type: TunnelType,
}

impl TunnelClose {
    pub fn src_address(&self) -> &UnifyAddress {
        &self.src_address
    }
    pub fn dest_address(&self) -> &UnifyAddress {
        &self.dest_address
    }
    pub fn tunnel_type(&self) -> TunnelType {
        self.tunnel_type
    }
}
