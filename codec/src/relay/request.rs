use crate::error::CodecError;
use ppaass_crypto::aes::{decrypt_with_aes, encrypt_with_aes};
use ppaass_domain::relay::RelayRequest;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
/// Relay request encoder is used by agent side after session init.
pub struct RelayRequestEncoder
{
    length_delimited_codec: LengthDelimitedCodec,
    aes_token: Vec<u8>,
}
impl RelayRequestEncoder {
    pub fn new(aes_token: Vec<u8>) -> Self {
        Self { length_delimited_codec: LengthDelimitedCodec::new(), aes_token }
    }
}
impl Encoder<RelayRequest> for RelayRequestEncoder
{
    type Error = CodecError;
    fn encode(&mut self, item: RelayRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let relay_request_bytes = bincode::serialize(&item)?;
        let relay_request_bytes = encrypt_with_aes(self.aes_token.as_slice(), &relay_request_bytes)?;
        Ok(self.length_delimited_codec.encode(relay_request_bytes.into(), dst)?)
    }
}
/// Relay request decoder is used by proxy side after session init.
pub struct RelayRequestDecoder {
    length_delimited_codec: LengthDelimitedCodec,
    aes_token: Vec<u8>,
}
impl RelayRequestDecoder {
    pub fn new(aes_token: Vec<u8>) -> Self {
        Self { length_delimited_codec: LengthDelimitedCodec::new(), aes_token }
    }
}
impl Decoder for RelayRequestDecoder {
    type Item = RelayRequest;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let relay_request_bytes = self.length_delimited_codec.decode(src)?;
        match relay_request_bytes {
            None => Ok(None),
            Some(relay_request_bytes) => {
                let relay_request_bytes = decrypt_with_aes(&self.aes_token, &relay_request_bytes)?;
                let relay_request = bincode::deserialize::<RelayRequest>(&relay_request_bytes)?;
                Ok(Some(relay_request))
            }
        }
    }
}