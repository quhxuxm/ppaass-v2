pub mod error;
mod heartbeat;
mod holder;
mod tunnel;
use crate::error::CodecError;
use crate::heartbeat::ping::{HeartbeatPingDecoder, HeartbeatPingEncoder};
use crate::heartbeat::pong::{HeartbeatPongDecoder, HeartbeatPongEncoder};
pub use holder::EncryptionHolder;
pub use holder::RsaCryptoHolder;
use ppaass_domain::{AgentPacket, ProxyPacket};
use std::sync::Arc;
use tokio_util::bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
pub use tunnel::*;
pub struct AgentPacketEncoder<F>
where
    F: RsaCryptoHolder,
{
    tunnel_init_request_encoder: TunnelInitRequestEncoder<F>,
    heartbeat_ping_encoder: HeartbeatPingEncoder,
    length_delimited_codec: LengthDelimitedCodec,
}

impl<F> AgentPacketEncoder<F>
where
    F: RsaCryptoHolder,
{
    pub fn new(rsa_crypto_holder: Arc<F>) -> Self {
        Self {
            tunnel_init_request_encoder: TunnelInitRequestEncoder::new(rsa_crypto_holder),
            heartbeat_ping_encoder: HeartbeatPingEncoder::new(),
            length_delimited_codec: Default::default(),
        }
    }
}

impl<F> Encoder<AgentPacket> for AgentPacketEncoder<F>
where
    F: RsaCryptoHolder,
{
    type Error = CodecError;
    fn encode(&mut self, item: AgentPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            AgentPacket::TunnelInit(tunnel_init_request) => {
                dst.put_u8(0);
                self.tunnel_init_request_encoder
                    .encode(tunnel_init_request, dst)
            }
            AgentPacket::Heartbeat(heartbeat_ping) => {
                dst.put_u8(1);
                self.heartbeat_ping_encoder.encode(heartbeat_ping, dst)
            }
            AgentPacket::Relay(relay_data) => {
                dst.put_u8(2);
                Ok(self.length_delimited_codec.encode(relay_data.into(), dst)?)
            }
        }
    }
}

pub struct AgentPacketDecoder<F>
where
    F: RsaCryptoHolder,
{
    tunnel_init_request_decoder: TunnelInitRequestDecoder<F>,
    heartbeat_ping_decoder: HeartbeatPingDecoder,
    length_delimited_codec: LengthDelimitedCodec,
}

impl<F> AgentPacketDecoder<F>
where
    F: RsaCryptoHolder,
{
    pub fn new(rsa_crypto_holder: Arc<F>) -> Self {
        Self {
            tunnel_init_request_decoder: TunnelInitRequestDecoder::new(rsa_crypto_holder),
            heartbeat_ping_decoder: HeartbeatPingDecoder::new(),
            length_delimited_codec: Default::default(),
        }
    }
}

impl<F> Decoder for AgentPacketDecoder<F>
where
    F: RsaCryptoHolder,
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
                let tunnel_init_request = self.tunnel_init_request_decoder.decode(src)?;
                match tunnel_init_request {
                    None => Ok(None),
                    Some(tunnel_init_request) => {
                        Ok(Some(AgentPacket::TunnelInit(tunnel_init_request)))
                    }
                }
            }
            1 => {
                let heartbeat_ping = self.heartbeat_ping_decoder.decode(src)?;
                match heartbeat_ping {
                    None => Ok(None),
                    Some(heartbeat_ping) => Ok(Some(AgentPacket::Heartbeat(heartbeat_ping))),
                }
            }
            2 => {
                let relay_data = self.length_delimited_codec.decode(src)?;
                match relay_data {
                    None => Ok(None),
                    Some(relay_data) => Ok(Some(AgentPacket::Relay(relay_data.to_vec()))),
                }
            }
            packet_type => Err(CodecError::InvalidAgentPacketByte(packet_type)),
        }
    }
}

///////////////////////////////

pub struct ProxyPacketEncoder<F>
where
    F: RsaCryptoHolder,
{
    tunnel_init_response_encoder: TunnelInitResponseEncoder<F>,
    heartbeat_pong_encoder: HeartbeatPongEncoder,
    length_delimited_codec: LengthDelimitedCodec,
}

impl<F> ProxyPacketEncoder<F>
where
    F: RsaCryptoHolder,
{
    pub fn new(rsa_crypto_holder: Arc<F>) -> Self {
        Self {
            tunnel_init_response_encoder: TunnelInitResponseEncoder::new(rsa_crypto_holder),
            heartbeat_pong_encoder: HeartbeatPongEncoder::new(),
            length_delimited_codec: Default::default(),
        }
    }
}

impl<F> Encoder<ProxyPacket> for ProxyPacketEncoder<F>
where
    F: RsaCryptoHolder,
{
    type Error = CodecError;
    fn encode(&mut self, item: ProxyPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            ProxyPacket::TunnelInit((auth_token, tunnel_init_response)) => {
                dst.put_u8(0);
                self.tunnel_init_response_encoder
                    .encode((auth_token, tunnel_init_response), dst)
            }
            ProxyPacket::Heartbeat(heartbeat_ping) => {
                dst.put_u8(1);
                self.heartbeat_pong_encoder.encode(heartbeat_ping, dst)
            }
            ProxyPacket::Relay(relay_data) => {
                dst.put_u8(2);
                Ok(self.length_delimited_codec.encode(relay_data.into(), dst)?)
            }
        }
    }
}

pub struct ProxyPacketDecoder<F>
where
    F: RsaCryptoHolder,
{
    tunnel_init_response_decoder: TunnelInitResponseDecoder<F>,
    heartbeat_pong_decoder: HeartbeatPongDecoder,
    length_delimited_codec: LengthDelimitedCodec,
    auth_token: String,
}

impl<F> ProxyPacketDecoder<F>
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
            length_delimited_codec: Default::default(),
            auth_token,
        }
    }
}

impl<F> Decoder for ProxyPacketDecoder<F>
where
    F: RsaCryptoHolder,
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
                let tunnel_init_response = self.tunnel_init_response_decoder.decode(src)?;
                match tunnel_init_response {
                    None => Ok(None),
                    Some(tunnel_init_response) => Ok(Some(ProxyPacket::TunnelInit((
                        self.auth_token.clone(),
                        tunnel_init_response,
                    )))),
                }
            }
            1 => {
                let heartbeat_pong = self.heartbeat_pong_decoder.decode(src)?;
                match heartbeat_pong {
                    None => Ok(None),
                    Some(heartbeat_pong) => Ok(Some(ProxyPacket::Heartbeat(heartbeat_pong))),
                }
            }
            2 => {
                let relay_data = self.length_delimited_codec.decode(src)?;
                match relay_data {
                    None => Ok(None),
                    Some(relay_data) => Ok(Some(ProxyPacket::Relay(relay_data.to_vec()))),
                }
            }
            packet_type => Err(CodecError::InvalidAgentPacketByte(packet_type)),
        }
    }
}
