use crate::error::ServerError;
use bytes::BufMut;
use ppaass_v2_domain::message::PpaassMessagePacket;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::Encoder;
pub struct AgentEdgeEncoder {}

impl AgentEdgeEncoder {
    pub fn new() -> Self {
        Self {}
    }
}
impl Encoder<PpaassMessagePacket> for AgentEdgeEncoder {
    type Error = ServerError;
    fn encode(&mut self, item: PpaassMessagePacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let body_bytes = bincode::serialize(&item)?;
        let body_length = body_bytes.len() as u64;
        dst.put_u64(body_length);
        dst.put(&body_bytes[..]);
        Ok(())
    }
}
