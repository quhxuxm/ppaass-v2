use crate::error::CodecError;
use bytes::{Bytes, BytesMut};
use ppaass_domain::packet::PpaassPacket;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
#[derive(Default)]
pub struct PpaassPacketCodec {
    length_delimited_codec: LengthDelimitedCodec,
}

impl Encoder<PpaassPacket> for PpaassPacketCodec {
    type Error = CodecError;
    fn encode(&mut self, item: PpaassPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let packet_bytes: Bytes = item.try_into()?;
        self.length_delimited_codec.encode(packet_bytes, dst)?;
        Ok(())
    }
}

impl Decoder for PpaassPacketCodec {
    type Item = PpaassPacket;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let received_bytes = self.length_delimited_codec.decode(src)?;
        match received_bytes {
            None => Ok(None),
            Some(received_bytes) => {
                let packet: PpaassPacket = received_bytes.try_into()?;
                Ok(Some(packet))
            }
        }
    }
}
