mod decoder;
mod encoder;
use crate::codec::decoder::AgentEdgeDecoder;
use crate::codec::encoder::AgentEdgeEncoder;
use crate::error::ServerError;
use bytes::BytesMut;
use ppaass_v2_domain::message::PpaassMessagePacket;
use tokio_util::codec::{Decoder, Encoder};
pub struct AgentEdgeCodec {
    encoder: AgentEdgeEncoder,
    decoder: AgentEdgeDecoder,
}

impl AgentEdgeCodec {
    pub fn new() -> Self {
        Self {
            encoder: AgentEdgeEncoder::new(),
            decoder: AgentEdgeDecoder::new(),
        }
    }
}

impl Encoder<PpaassMessagePacket> for AgentEdgeCodec {
    type Error = ServerError;
    fn encode(&mut self, item: PpaassMessagePacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.encoder.encode(item, dst)
    }
}

impl Decoder for AgentEdgeCodec {
    type Item = PpaassMessagePacket;
    type Error = ServerError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.decoder.decode(src)
    }
}
