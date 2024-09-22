use crate::error::ServerError;
use bytes::Bytes;
use ppaass_v2_domain::message::PpaassMessagePacket;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};
enum AgentEdgeDecoderState {
    Header,
    Body(Bytes),
}
pub struct AgentEdgeDecoder {
    state: AgentEdgeDecoderState,
}

impl Decoder for AgentEdgeDecoder {
    type Item = PpaassMessagePacket;
    type Error = ServerError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match &self.state {
            AgentEdgeDecoderState::Header => {}
            AgentEdgeDecoderState::Body(body_bytes) => {}
        }
        todo!()
    }
}

pub struct AgentEdgeEncoder {}

impl Encoder<PpaassMessagePacket> for AgentEdgeEncoder {
    type Error = ServerError;
    fn encode(&mut self, item: PpaassMessagePacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        todo!()
    }
}
