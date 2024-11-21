use crate::error::CodecError;
use ppaass_crypto::aes::{decrypt_with_aes, encrypt_with_aes};
use ppaass_domain::relay::RelayResponse;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
/// Relay response encoder is used by proxy side after session init.
pub struct RelayResponseEncoder {
    length_delimited_codec: LengthDelimitedCodec,
    aes_token: Vec<u8>,
}
impl RelayResponseEncoder {
    pub fn new(aes_token: Vec<u8>) -> Self {
        Self { length_delimited_codec: LengthDelimitedCodec::new(), aes_token }
    }
}
impl Encoder<RelayResponse> for RelayResponseEncoder
{
    type Error = CodecError;
    fn encode(&mut self, item: RelayResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let relay_response_bytes = bincode::serialize(&item)?;
        let relay_response_bytes = encrypt_with_aes(self.aes_token.as_slice(), &relay_response_bytes)?;
        Ok(self.length_delimited_codec.encode(relay_response_bytes.into(), dst)?)
    }
}
/// Relay response decoder is used by agent side after session init.
pub struct RelayResponseDecoder {
    length_delimited_codec: LengthDelimitedCodec,
    aes_token: Vec<u8>,
}
impl crate::RelayResponseDecoder {
    pub fn new(aes_token: Vec<u8>) -> Self {
        Self { length_delimited_codec: LengthDelimitedCodec::new(), aes_token }
    }
}
impl Decoder for RelayResponseDecoder {
    type Item = RelayResponse;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let relay_response_bytes = self.length_delimited_codec.decode(src)?;
        match relay_response_bytes {
            None => Ok(None),
            Some(relay_response_bytes) => {
                let relay_response_bytes = decrypt_with_aes(&self.aes_token, &relay_response_bytes)?;
                let relay_response = bincode::deserialize::<RelayResponse>(&relay_response_bytes)?;
                Ok(Some(relay_response))
            }
        }
    }
}
