use crate::error::CodecError;
use ppaass_domain::heartbeat::HeartbeatPong;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
/// Tunnel init request encoder will be used by agent side
pub struct HeartbeatPongEncoder {
    length_delimited_codec: LengthDelimitedCodec,
}
impl HeartbeatPongEncoder {
    pub fn new() -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
        }
    }
}
impl Encoder<HeartbeatPong> for HeartbeatPongEncoder {
    type Error = CodecError;
    fn encode(&mut self, item: HeartbeatPong, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let ping_bytes = bincode::serialize(&item)?;
        Ok(self.length_delimited_codec.encode(ping_bytes.into(), dst)?)
    }
}
/// Tunnel init request decoder will be used by proxy side
pub struct HeartbeatPongDecoder {
    length_delimited_codec: LengthDelimitedCodec,
}
impl HeartbeatPongDecoder {
    pub fn new() -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
        }
    }
}
impl Decoder for HeartbeatPongDecoder {
    type Item = HeartbeatPong;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let ping_request_bytes = self.length_delimited_codec.decode(src)?;
        match ping_request_bytes {
            None => Ok(None),
            Some(ping_request_bytes) => Ok(Some(bincode::deserialize::<HeartbeatPong>(
                &ping_request_bytes,
            )?)),
        }
    }
}
