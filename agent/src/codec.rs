use crate::crypto::AgentRsaCryptoHolder;
use crate::error::AgentError;
use bytes::BytesMut;
use ppaass_codec::{AgentControlPacketEncoder, AgentDataPacketEncoder, ProxyControlPacketDecoder, ProxyDataPacketDecoder};
use ppaass_domain::tunnel::Encryption;
use ppaass_domain::{AgentControlPacket, AgentDataPacket, ProxyControlPacket, ProxyDataPacket};
use std::sync::Arc;
use tokio_util::codec::{Decoder, Encoder};
pub struct ControlPacketCodec {
    agent_control_packet_encoder: AgentControlPacketEncoder<AgentRsaCryptoHolder>,
    proxy_control_packet_decoder: ProxyControlPacketDecoder<AgentRsaCryptoHolder>,
}
impl ControlPacketCodec {
    pub fn new(auth_token: String, rsa_crypto_holder: Arc<AgentRsaCryptoHolder>) -> Self {
        Self {
            agent_control_packet_encoder: AgentControlPacketEncoder::new(rsa_crypto_holder.clone()),
            proxy_control_packet_decoder: ProxyControlPacketDecoder::new(auth_token, rsa_crypto_holder),
        }
    }
}
impl Encoder<AgentControlPacket> for ControlPacketCodec {
    type Error = AgentError;
    fn encode(&mut self, item: AgentControlPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.agent_control_packet_encoder.encode(item, dst)?;
        Ok(())
    }
}
impl Decoder for ControlPacketCodec {
    type Item = ProxyControlPacket;
    type Error = AgentError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.proxy_control_packet_decoder.decode(src)?)
    }
}
pub struct DataPacketCodec {
    agent_data_packet_encoder: AgentDataPacketEncoder,
    proxy_data_packet_decoder: ProxyDataPacketDecoder,
}
impl DataPacketCodec {
    pub fn new(agent_encryption: Encryption, proxy_encryption: Encryption) -> Self {
        Self {
            agent_data_packet_encoder: AgentDataPacketEncoder::new(agent_encryption),
            proxy_data_packet_decoder: ProxyDataPacketDecoder::new(proxy_encryption),
        }
    }
}
impl Encoder<AgentDataPacket> for DataPacketCodec {
    type Error = AgentError;
    fn encode(&mut self, item: AgentDataPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(self.agent_data_packet_encoder.encode(item, dst)?)
    }
}
impl Decoder for DataPacketCodec {
    type Item = ProxyDataPacket;
    type Error = AgentError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.proxy_data_packet_decoder.decode(src)?)
    }
}

