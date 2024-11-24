use crate::error::CodecError;
use ppaass_domain::heartbeat::HeartbeatPing;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
/// Tunnel init request encoder will be used by agent side
pub struct HeartbeatPingEncoder {
    length_delimited_codec: LengthDelimitedCodec,
}
impl HeartbeatPingEncoder {
    pub fn new() -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
        }
    }
}
impl Encoder<HeartbeatPing> for HeartbeatPingEncoder {
    type Error = CodecError;
    fn encode(&mut self, item: HeartbeatPing, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let ping_bytes = bincode::serialize(&item)?;
        Ok(self.length_delimited_codec.encode(ping_bytes.into(), dst)?)
    }
}
/// Tunnel init request decoder will be used by proxy side
pub struct HeartbeatPingDecoder {
    length_delimited_codec: LengthDelimitedCodec,
}
impl HeartbeatPingDecoder {
    pub fn new() -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
        }
    }
}
impl Decoder for HeartbeatPingDecoder {
    type Item = HeartbeatPing;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let ping_request_bytes = self.length_delimited_codec.decode(src)?;
        match ping_request_bytes {
            None => Ok(None),
            Some(ping_request_bytes) => Ok(Some(bincode::deserialize::<HeartbeatPing>(
                &ping_request_bytes,
            )?)),
        }
    }
}
