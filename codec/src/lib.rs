mod session;
mod relay;
pub mod error;
mod holder;
use crate::error::CodecError;
use crate::holder::{EncryptionHolder, RsaCryptoHolder};
use ppaass_domain::{AgentPacket, ProxyPacket};
pub use relay::*;
pub use session::*;
use std::sync::Arc;
use tokio_util::bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
pub struct AgentPacketEncoder<F, T>
where
    F: RsaCryptoHolder,
    T: EncryptionHolder,
{
    session_init_request_encoder: SessionInitRequestEncoder<F>,
    relay_request_encoder: RelayRequestEncoder<T>,
}
impl<F, T> AgentPacketEncoder<F, T>
where
    F: RsaCryptoHolder,
    T: EncryptionHolder,
{
    pub fn new(rsa_crypto_holder: Arc<F>, encryption_holder: Arc<T>) -> Self {
        Self {
            session_init_request_encoder: SessionInitRequestEncoder::new(rsa_crypto_holder),
            relay_request_encoder: RelayRequestEncoder::new(encryption_holder),
        }
    }
}
impl<F, T> Encoder<AgentPacket> for AgentPacketEncoder<F, T>
where
    F: RsaCryptoHolder,
    T: EncryptionHolder,
{
    type Error = CodecError;
    fn encode(&mut self, item: AgentPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            AgentPacket::SessionInit(session_init_request) => {
                dst.put_u8(0);
                self.session_init_request_encoder.encode(session_init_request, dst)
            }
            AgentPacket::Relay(relay_request) => {
                dst.put_u8(1);
                self.relay_request_encoder.encode(relay_request, dst)
            }
        }
    }
}
pub struct AgentPacketDecoder<F, T>
where
    F: RsaCryptoHolder,
    T: EncryptionHolder,
{
    session_init_request_decoder: SessionInitRequestDecoder<F>,
    relay_request_decoder: RelayRequestDecoder<T>,
}
impl<F, T> AgentPacketDecoder<F, T>
where
    F: RsaCryptoHolder,
    T: EncryptionHolder,
{
    pub fn new(rsa_crypto_holder: Arc<F>, encryption_holder: Arc<T>) -> Self {
        Self {
            session_init_request_decoder: SessionInitRequestDecoder::new(rsa_crypto_holder),
            relay_request_decoder: RelayRequestDecoder::new(encryption_holder),
        }
    }
}
impl<F, T> Decoder for AgentPacketDecoder<F, T>
where
    F: RsaCryptoHolder,
    T: EncryptionHolder,
{
    type Item = AgentPacket;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < size_of::<u8>() {
            return Ok(None);
        }
        let packet_type = src.get_u8();
        match packet_type {
            0 => {
                Ok(self.session_init_request_decoder.decode(src)?.map(AgentPacket::SessionInit))
            }
            1 => {
                Ok(self.relay_request_decoder.decode(src)?.map(AgentPacket::Relay))
            }
            v => {
                Err(CodecError::InvalidAgentPacketByte(v))
            }
        }
    }
}
pub struct ProxyPacketEncoder<F, T>
where
    F: RsaCryptoHolder,
    T: EncryptionHolder,
{
    session_init_response_encoder: SessionInitResponseEncoder<F>,
    relay_response_encoder: RelayResponseEncoder<T>,
}
impl<F, T> ProxyPacketEncoder<F, T>
where
    F: RsaCryptoHolder,
    T: EncryptionHolder,
{
    pub fn new(rsa_crypto_holder: Arc<F>, encryption_holder: Arc<T>) -> Self {
        Self {
            session_init_response_encoder: SessionInitResponseEncoder::new(rsa_crypto_holder),
            relay_response_encoder: RelayResponseEncoder::new(encryption_holder),
        }
    }
}
impl<F, T> Encoder<(String, ProxyPacket)> for ProxyPacketEncoder<F, T>
where
    F: RsaCryptoHolder,
    T: EncryptionHolder,
{
    type Error = CodecError;
    fn encode(&mut self, item: (String, ProxyPacket), dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (auth_token, item) = item;
        match item {
            ProxyPacket::SessionInit(session_init_response) => {
                dst.put_u8(0);
                self.session_init_response_encoder.encode((auth_token, session_init_response), dst)
            }
            ProxyPacket::Relay(relay_response) => {
                dst.put_u8(1);
                self.relay_response_encoder.encode(relay_response, dst)
            }
        }
    }
}
pub struct ProxyPacketDecoder<F, T>
where
    F: RsaCryptoHolder,
    T: EncryptionHolder,
{
    session_init_response_decoder: SessionInitResponseDecoder<F>,
    relay_response_decoder: RelayResponseDecoder<T>,
}
impl<F, T> ProxyPacketDecoder<F, T>
where
    F: RsaCryptoHolder,
    T: EncryptionHolder,
{
    pub fn new(auth_token: String, rsa_crypto_holder: Arc<F>, encryption_holder: Arc<T>) -> Self {
        Self {
            session_init_response_decoder: SessionInitResponseDecoder::new(auth_token, rsa_crypto_holder),
            relay_response_decoder: RelayResponseDecoder::new(encryption_holder),
        }
    }
}
impl<F, T> Decoder for ProxyPacketDecoder<F, T>
where
    F: RsaCryptoHolder,
    T: EncryptionHolder,
{
    type Item = ProxyPacket;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < size_of::<u8>() {
            return Ok(None);
        }
        let packet_type = src.get_u8();
        match packet_type {
            0 => {
                Ok(self.session_init_response_decoder.decode(src)?.map(ProxyPacket::SessionInit))
            }
            1 => {
                Ok(self.relay_response_decoder.decode(src)?.map(ProxyPacket::Relay))
            }
            v => {
                Err(CodecError::InvalidAgentPacketByte(v))
            }
        }
    }
}