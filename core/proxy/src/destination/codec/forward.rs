use crate::crypto::ProxyRsaCryptoHolder;
use crate::error::ProxyError;
use bytes::BytesMut;
use ppaass_codec::{
    AgentControlPacketEncoder, AgentDataPacketEncoder, ProxyControlPacketDecoder,
    ProxyDataPacketDecoder,
};
use ppaass_domain::tunnel::Encryption;
use ppaass_domain::{AgentControlPacket, AgentDataPacket, ProxyControlPacket, ProxyDataPacket};
use std::sync::Arc;
use tokio_util::codec::{Decoder, Encoder};
pub struct ForwardDestinationTransportControlPacketCodec {
    agent_control_packet_encoder: AgentControlPacketEncoder<ProxyRsaCryptoHolder>,
    proxy_control_packet_decoder: ProxyControlPacketDecoder<ProxyRsaCryptoHolder>,
}
impl ForwardDestinationTransportControlPacketCodec {
    pub fn new(
        forward_auth_token: String,
        rsa_crypto_holder: Arc<ProxyRsaCryptoHolder>,
    ) -> Self {
        Self {
            agent_control_packet_encoder: AgentControlPacketEncoder::new(rsa_crypto_holder.clone()),
            proxy_control_packet_decoder: ProxyControlPacketDecoder::new(
                forward_auth_token,
                rsa_crypto_holder,
            ),
        }
    }
}
impl Encoder<AgentControlPacket> for ForwardDestinationTransportControlPacketCodec {
    type Error = ProxyError;
    fn encode(&mut self, item: AgentControlPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(self.agent_control_packet_encoder.encode(item, dst)?)
    }
}
impl Decoder for ForwardDestinationTransportControlPacketCodec {
    type Item = ProxyControlPacket;
    type Error = ProxyError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.proxy_control_packet_decoder.decode(src)?)
    }
}
pub struct ForwardDestinationTransportDataPacketCodec {
    agent_data_packet_encoder: AgentDataPacketEncoder,
    proxy_data_packet_decoder: ProxyDataPacketDecoder,
}
impl ForwardDestinationTransportDataPacketCodec {
    pub fn new(agent_encryption: Encryption, proxy_encryption: Encryption) -> Self {
        Self {
            agent_data_packet_encoder: AgentDataPacketEncoder::new(agent_encryption),
            proxy_data_packet_decoder: ProxyDataPacketDecoder::new(proxy_encryption),
        }
    }
}
impl Encoder<AgentDataPacket> for ForwardDestinationTransportDataPacketCodec {
    type Error = ProxyError;
    fn encode(&mut self, item: AgentDataPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(self.agent_data_packet_encoder.encode(item, dst)?)
    }
}
impl Decoder for ForwardDestinationTransportDataPacketCodec {
    type Item = ProxyDataPacket;
    type Error = ProxyError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.proxy_data_packet_decoder.decode(src)?)
    }
}
