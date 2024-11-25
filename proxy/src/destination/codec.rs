use crate::destination::DestinationDataPacket;
use crate::error::ProxyError;
use bytes::BytesMut;
use tokio_util::codec::{BytesCodec, Decoder, Encoder};
pub struct DestinationDataTcpCodec {
    bytes_codec: BytesCodec,
}
impl DestinationDataTcpCodec {
    pub fn new() -> Self {
        Self {
            bytes_codec: BytesCodec::new(),
        }
    }
}
impl Decoder for DestinationDataTcpCodec {
    type Item = DestinationDataPacket;
    type Error = ProxyError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let destination_data = self.bytes_codec.decode(src)?;
        match destination_data {
            None => {
                Ok(None)
            }
            Some(data) => {
                Ok(Some(DestinationDataPacket::Tcp(data.to_vec())))
            }
        }
    }
}
impl Encoder<BytesMut> for DestinationDataTcpCodec {
    type Error = ProxyError;
    fn encode(&mut self, item: BytesMut, dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(self.bytes_codec.encode(item, dst)?)
    }
}