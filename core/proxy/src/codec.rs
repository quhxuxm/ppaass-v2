use crate::crypto::ProxyRsaCryptoHolder;
use crate::error::ProxyError;
use bytes::BytesMut;
use ppaass_codec::{
    AgentControlPacketDecoder, AgentDataPacketDecoder, ProxyControlPacketEncoder,
    ProxyDataPacketEncoder,
};
use ppaass_domain::tunnel::Encryption;
use ppaass_domain::{AgentControlPacket, AgentDataPacket, ProxyControlPacket, ProxyDataPacket};
use std::sync::Arc;
use tokio_util::codec::{Decoder, Encoder};
pub struct ControlPacketCodec {
    agent_control_packet_decoder: AgentControlPacketDecoder<ProxyRsaCryptoHolder>,
    proxy_control_packet_encoder: ProxyControlPacketEncoder<ProxyRsaCryptoHolder>,
}
impl ControlPacketCodec {
    pub fn new(rsa_crypto_holder: Arc<ProxyRsaCryptoHolder>) -> Self {
        Self {
            agent_control_packet_decoder: AgentControlPacketDecoder::new(rsa_crypto_holder.clone()),
            proxy_control_packet_encoder: ProxyControlPacketEncoder::new(rsa_crypto_holder),
        }
    }
}
impl Encoder<ProxyControlPacket> for ControlPacketCodec {
    type Error = ProxyError;
    fn encode(&mut self, item: ProxyControlPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.proxy_control_packet_encoder.encode(item, dst)?;
        Ok(())
    }
}
impl Decoder for ControlPacketCodec {
    type Item = AgentControlPacket;
    type Error = ProxyError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.agent_control_packet_decoder.decode(src)?)
    }
}
pub struct DataPacketCodec {
    agent_data_packet_decoder: AgentDataPacketDecoder,
    proxy_data_packet_encoder: ProxyDataPacketEncoder,
}
impl DataPacketCodec {
    pub fn new(agent_encryption: Encryption, proxy_encryption: Encryption) -> Self {
        Self {
            agent_data_packet_decoder: AgentDataPacketDecoder::new(agent_encryption),
            proxy_data_packet_encoder: ProxyDataPacketEncoder::new(proxy_encryption),
        }
    }
}
impl Encoder<ProxyDataPacket> for DataPacketCodec {
    type Error = ProxyError;
    fn encode(&mut self, item: ProxyDataPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(self.proxy_data_packet_encoder.encode(item, dst)?)
    }
}
impl Decoder for DataPacketCodec {
    type Item = AgentDataPacket;
    type Error = ProxyError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.agent_data_packet_decoder.decode(src)?)
    }
}
