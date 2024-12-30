mod codec;
mod tcp;
mod udp;
pub use codec::DestinationDataTcpCodec;
pub use tcp::new_tcp_destination;
pub use udp::new_udp_destination;
