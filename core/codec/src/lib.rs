pub mod error;
mod heartbeat;
mod holder;
mod tunnel;
use crate::error::CodecError;
use crate::heartbeat::ping::{HeartbeatPingDecoder, HeartbeatPingEncoder};
use crate::heartbeat::pong::{HeartbeatPongDecoder, HeartbeatPongEncoder};
pub use holder::EncryptionHolder;
pub use holder::RsaCryptoHolder;
use ppaass_crypto::aes::{decrypt_with_aes, encrypt_with_aes};
use ppaass_domain::tunnel::Encryption;
use ppaass_domain::{AgentControlPacket, AgentDataPacket, ProxyControlPacket, ProxyDataPacket};
use std::sync::Arc;
use tokio_util::bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
pub use tunnel::*;
pub struct AgentControlPacketEncoder<F>
where
    F: RsaCryptoHolder,
{
    tunnel_init_request_encoder: TunnelInitRequestEncoder<F>,
    heartbeat_ping_encoder: HeartbeatPingEncoder,
}
impl<F> AgentControlPacketEncoder<F>
where
    F: RsaCryptoHolder,
{
    pub fn new(rsa_crypto_holder: Arc<F>) -> Self {
        Self {
            tunnel_init_request_encoder: TunnelInitRequestEncoder::new(rsa_crypto_holder),
            heartbeat_ping_encoder: HeartbeatPingEncoder::new(),
        }
    }
}
impl<F> Encoder<AgentControlPacket> for AgentControlPacketEncoder<F>
where
    F: RsaCryptoHolder,
{
    type Error = CodecError;
    fn encode(&mut self, item: AgentControlPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            AgentControlPacket::TunnelInit(tunnel_init_request) => {
                dst.put_u8(0);
                self.tunnel_init_request_encoder
                    .encode(tunnel_init_request, dst)
            }
            AgentControlPacket::Heartbeat(heartbeat_ping) => {
                dst.put_u8(1);
                self.heartbeat_ping_encoder.encode(heartbeat_ping, dst)
            }
        }
    }
}
pub struct AgentControlPacketDecoder<F>
where
    F: RsaCryptoHolder,
{
    tunnel_init_request_decoder: TunnelInitRequestDecoder<F>,
    heartbeat_ping_decoder: HeartbeatPingDecoder,
}
impl<F> AgentControlPacketDecoder<F>
where
    F: RsaCryptoHolder,
{
    pub fn new(rsa_crypto_holder: Arc<F>) -> Self {
        Self {
            tunnel_init_request_decoder: TunnelInitRequestDecoder::new(rsa_crypto_holder),
            heartbeat_ping_decoder: HeartbeatPingDecoder::new(),
        }
    }
}
impl<F> Decoder for AgentControlPacketDecoder<F>
where
    F: RsaCryptoHolder,
{
    type Item = AgentControlPacket;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < size_of::<u8>() {
            return Ok(None);
        }
        let packet_type = src.get_u8();
        match packet_type {
            0 => {
                let tunnel_init_request = self.tunnel_init_request_decoder.decode(src)?;
                match tunnel_init_request {
                    None => Ok(None),
                    Some(tunnel_init_request) => {
                        Ok(Some(AgentControlPacket::TunnelInit(tunnel_init_request)))
                    }
                }
            }
            1 => {
                let heartbeat_ping = self.heartbeat_ping_decoder.decode(src)?;
                match heartbeat_ping {
                    None => Ok(None),
                    Some(heartbeat_ping) => Ok(Some(AgentControlPacket::Heartbeat(heartbeat_ping))),
                }
            }
            packet_type => Err(CodecError::InvalidAgentPacketByte(packet_type)),
        }
    }
}
pub struct ProxyControlPacketEncoder<F>
where
    F: RsaCryptoHolder,
{
    tunnel_init_response_encoder: TunnelInitResponseEncoder<F>,
    heartbeat_pong_encoder: HeartbeatPongEncoder,
}
impl<F> ProxyControlPacketEncoder<F>
where
    F: RsaCryptoHolder,
{
    pub fn new(rsa_crypto_holder: Arc<F>) -> Self {
        Self {
            tunnel_init_response_encoder: TunnelInitResponseEncoder::new(rsa_crypto_holder),
            heartbeat_pong_encoder: HeartbeatPongEncoder::new(),
        }
    }
}
impl<F> Encoder<ProxyControlPacket> for ProxyControlPacketEncoder<F>
where
    F: RsaCryptoHolder,
{
    type Error = CodecError;
    fn encode(&mut self, item: ProxyControlPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            ProxyControlPacket::TunnelInit((auth_token, tunnel_init_response)) => {
                dst.put_u8(0);
                self.tunnel_init_response_encoder
                    .encode((auth_token, tunnel_init_response), dst)
            }
            ProxyControlPacket::Heartbeat(heartbeat_ping) => {
                dst.put_u8(1);
                self.heartbeat_pong_encoder.encode(heartbeat_ping, dst)
            }
        }
    }
}
pub struct ProxyControlPacketDecoder<F>
where
    F: RsaCryptoHolder,
{
    tunnel_init_response_decoder: TunnelInitResponseDecoder<F>,
    heartbeat_pong_decoder: HeartbeatPongDecoder,
    auth_token: String,
}
impl<F> ProxyControlPacketDecoder<F>
where
    F: RsaCryptoHolder,
{
    pub fn new(auth_token: String, rsa_crypto_holder: Arc<F>) -> Self {
        Self {
            tunnel_init_response_decoder: TunnelInitResponseDecoder::new(
                auth_token.clone(),
                rsa_crypto_holder,
            ),
            heartbeat_pong_decoder: HeartbeatPongDecoder::new(),
            auth_token,
        }
    }
}
impl<F> Decoder for ProxyControlPacketDecoder<F>
where
    F: RsaCryptoHolder,
{
    type Item = ProxyControlPacket;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < size_of::<u8>() {
            return Ok(None);
        }
        let packet_type = src.get_u8();
        match packet_type {
            0 => {
                let tunnel_init_response = self.tunnel_init_response_decoder.decode(src)?;
                match tunnel_init_response {
                    None => Ok(None),
                    Some(tunnel_init_response) => Ok(Some(ProxyControlPacket::TunnelInit((
                        self.auth_token.clone(),
                        tunnel_init_response,
                    )))),
                }
            }
            1 => {
                let heartbeat_pong = self.heartbeat_pong_decoder.decode(src)?;
                match heartbeat_pong {
                    None => Ok(None),
                    Some(heartbeat_pong) => Ok(Some(ProxyControlPacket::Heartbeat(heartbeat_pong))),
                }
            }
            packet_type => Err(CodecError::InvalidAgentPacketByte(packet_type)),
        }
    }
}
pub struct AgentDataPacketEncoder {
    length_delimited_codec: LengthDelimitedCodec,
    encryption: Encryption,
}
impl AgentDataPacketEncoder {
    pub fn new(encryption: Encryption) -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
            encryption,
        }
    }
}
impl Encoder<AgentDataPacket> for AgentDataPacketEncoder {
    type Error = CodecError;
    fn encode(&mut self, item: AgentDataPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let encrypted_data = match &self.encryption {
            Encryption::Plain => bincode::serialize(&item)?,
            Encryption::Aes(aes_token) => {
                let raw = bincode::serialize(&item)?;
                encrypt_with_aes(aes_token, &raw)?
            }
        };
        Ok(self
            .length_delimited_codec
            .encode(encrypted_data.into(), dst)?)
    }
}
pub struct AgentDataPacketDecoder {
    length_delimited_codec: LengthDelimitedCodec,
    encryption: Encryption,
}
impl AgentDataPacketDecoder {
    pub fn new(encryption: Encryption) -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
            encryption,
        }
    }
}
impl Decoder for AgentDataPacketDecoder {
    type Item = AgentDataPacket;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let encrypted_data = self.length_delimited_codec.decode(src)?;
        match encrypted_data {
            None => Ok(None),
            Some(encrypted_data) => match &self.encryption {
                Encryption::Plain => Ok(Some(bincode::deserialize(&encrypted_data)?)),
                Encryption::Aes(aes_token) => {
                    let raw = decrypt_with_aes(aes_token, &encrypted_data)?;
                    Ok(Some(bincode::deserialize(&raw)?))
                }
            },
        }
    }
}
/////////////////////////
pub struct ProxyDataPacketEncoder {
    length_delimited_codec: LengthDelimitedCodec,
    encryption: Encryption,
}
impl ProxyDataPacketEncoder {
    pub fn new(encryption: Encryption) -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
            encryption,
        }
    }
}
impl Encoder<ProxyDataPacket> for ProxyDataPacketEncoder {
    type Error = CodecError;
    fn encode(&mut self, item: ProxyDataPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let encrypted_data = match &self.encryption {
            Encryption::Plain => bincode::serialize(&item)?,
            Encryption::Aes(aes_token) => {
                let raw = bincode::serialize(&item)?;
                encrypt_with_aes(aes_token, &raw)?
            }
        };
        Ok(self
            .length_delimited_codec
            .encode(encrypted_data.into(), dst)?)
    }
}
pub struct ProxyDataPacketDecoder {
    length_delimited_codec: LengthDelimitedCodec,
    encryption: Encryption,
}
impl ProxyDataPacketDecoder {
    pub fn new(encryption: Encryption) -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
            encryption,
        }
    }
}
impl Decoder for ProxyDataPacketDecoder {
    type Item = ProxyDataPacket;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let encrypted_data = self.length_delimited_codec.decode(src)?;
        match encrypted_data {
            None => Ok(None),
            Some(encrypted_data) => match &self.encryption {
                Encryption::Plain => Ok(Some(bincode::deserialize(&encrypted_data)?)),
                Encryption::Aes(aes_token) => {
                    let raw = decrypt_with_aes(aes_token, &encrypted_data)?;
                    Ok(Some(bincode::deserialize(&raw)?))
                }
            },
        }
    }
}
