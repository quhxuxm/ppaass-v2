use crate::crypto::ProxyRsaCryptoHolder;
use crate::error::ProxyError;
use bytes::BytesMut;
use ppaass_codec::{
    AgentPacketDecoder, ProxyPacketEncoder,
};
use ppaass_domain::{AgentPacket, ProxyPacket};
use std::sync::Arc;
use tokio_util::codec::{Decoder, Encoder};
pub struct AgentConnectionCodec {
    agent_packet_decoder: AgentPacketDecoder<ProxyRsaCryptoHolder>,
    proxy_packet_encoder: ProxyPacketEncoder<ProxyRsaCryptoHolder>,
}
impl AgentConnectionCodec {
    pub fn new(rsa_crypto_holder: Arc<ProxyRsaCryptoHolder>) -> Self {
        Self {
            agent_packet_decoder: AgentPacketDecoder::new(rsa_crypto_holder.clone()),
            proxy_packet_encoder: ProxyPacketEncoder::new(rsa_crypto_holder),
        }
    }
}
impl Encoder<ProxyPacket> for AgentConnectionCodec {
    type Error = ProxyError;
    fn encode(&mut self, item: ProxyPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.proxy_packet_encoder.encode(item, dst)?;
        Ok(())
    }
}
impl Decoder for AgentConnectionCodec {
    type Item = AgentPacket;
    type Error = ProxyError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.agent_packet_decoder.decode(src)?)
    }
}
