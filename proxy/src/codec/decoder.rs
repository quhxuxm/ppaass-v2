use crate::error::ServerError;
use bytes::Buf;
use ppaass_v2_domain::message::PpaassMessagePacket;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::Decoder;
enum AgentEdgeDecoderState {
    Header,
    Body(usize),
}

pub struct AgentEdgeDecoder {
    state: AgentEdgeDecoderState,
}

impl AgentEdgeDecoder {
    pub fn new() -> Self {
        Self {
            state: AgentEdgeDecoderState::Header,
        }
    }
}

impl Decoder for AgentEdgeDecoder {
    type Item = PpaassMessagePacket;
    type Error = ServerError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match &self.state {
            AgentEdgeDecoderState::Header => {
                if src.len() < size_of::<u64>() {
                    return Ok(None);
                }
                let length = src.get_u32();
                self.state = AgentEdgeDecoderState::Body(length as usize);
                Ok(None)
            }
            AgentEdgeDecoderState::Body(body_length) => {
                if src.len() < *body_length {
                    Ok(None)
                } else {
                    let body_bytes = src.split_to(*body_length);
                    let packet = bincode::deserialize::<PpaassMessagePacket>(&body_bytes)?;
                    self.state = AgentEdgeDecoderState::Header;
                    Ok(Some(packet))
                }
            }
        }
    }
}
