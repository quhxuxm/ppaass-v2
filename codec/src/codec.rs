use std::io::{Read, Write};
use crate::error::CodecError;
use bytes::{Bytes, BytesMut};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
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
        let  gz_encoder_buf = BytesMut::new();
        let mut gz_encoder=GzEncoder::new(gz_encoder_buf, Compression::fast());
        gz_encoder.write_all(&packet_bytes)?;
        let compressed_packet_bytes = gz_encoder.finish()?;
        self.length_delimited_codec.encode(compressed_packet_bytes.freeze(), dst)?;
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
                let mut gz_decoder = GzDecoder::new(received_bytes);
                let mut decompressed_received_bytes = Vec::new();
                gz_decoder.read_to_end(&mut decompressed_received_bytes)?;
                let packet: PpaassPacket = Bytes::from(decompressed_received_bytes).try_into()?;
                Ok(Some(packet))
            }
        }
    }
}
