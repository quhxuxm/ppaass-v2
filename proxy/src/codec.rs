use crate::error::ServerError;
use ppaass_v2_domain::message::PpaassMessagePacket;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};
pub struct AgentConnectionCodec {}

impl AgentConnectionCodec {
    pub fn new() -> Self {
        Self {}
    }
}
impl Decoder for AgentConnectionCodec {
    type Item = PpaassMessagePacket;
    type Error = ServerError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        todo!()
    }
}

impl Encoder<PpaassMessagePacket> for AgentConnectionCodec {
    type Error = ServerError;
    fn encode(&mut self, item: PpaassMessagePacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        todo!()
    }
}
