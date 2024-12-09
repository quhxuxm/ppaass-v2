mod forward;
mod raw;
use crate::destination::codec::forward::ForwardDestinationTransportDataPacketCodec;
use crate::destination::codec::raw::RawDestinationTransportCodec;
use crate::destination::DestinationDataPacket;
use crate::error::ProxyError;
use bytes::BytesMut;
use ppaass_domain::tunnel::Encryption;
use ppaass_domain::{AgentDataPacket, ProxyDataPacket};
use tokio_util::codec::{Decoder, Encoder};
pub use forward::ForwardDestinationTransportControlPacketCodec;
pub enum DestinationDataTcpCodec {
    Raw(RawDestinationTransportCodec),
    Forward(ForwardDestinationTransportDataPacketCodec),
}
impl DestinationDataTcpCodec {
    pub fn new_raw() -> Self {
        DestinationDataTcpCodec::Raw(RawDestinationTransportCodec::new())
    }

    pub fn new_forward(agent_encryption: Encryption, proxy_encryption: Encryption) -> Self {
        DestinationDataTcpCodec::Forward(ForwardDestinationTransportDataPacketCodec::new(
            agent_encryption,
            proxy_encryption,
        ))
    }
}
impl Decoder for DestinationDataTcpCodec {
    type Item = DestinationDataPacket;
    type Error = ProxyError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self {
            DestinationDataTcpCodec::Raw(raw_codec) => {
                let destination_data = raw_codec.decode(src)?;
                match destination_data {
                    None => Ok(None),
                    Some(data) => Ok(Some(DestinationDataPacket::Tcp(data.to_vec()))),
                }
            }
            DestinationDataTcpCodec::Forward(forward_codec) => {
                let destination_data = forward_codec.decode(src)?;
                match destination_data {
                    None => Ok(None),
                    Some(ProxyDataPacket::Tcp(data)) => Ok(Some(DestinationDataPacket::Tcp(data))),
                    Some(ProxyDataPacket::Udp { .. }) => Err(ProxyError::InvalidData),
                }
            }
        }
    }
}
impl Encoder<BytesMut> for DestinationDataTcpCodec {
    type Error = ProxyError;
    fn encode(&mut self, item: BytesMut, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match self {
            DestinationDataTcpCodec::Raw(raw_codec) => Ok(raw_codec.encode(item, dst)?),
            DestinationDataTcpCodec::Forward(forward_codec) => {
                Ok(forward_codec.encode(AgentDataPacket::Tcp(item.to_vec()), dst)?)
            }
        }
    }
}
