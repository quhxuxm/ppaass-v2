use crate::error::CodecError;
use crate::EncryptionHolder;
use ppaass_crypto::aes::{decrypt_with_aes, encrypt_with_aes};
use ppaass_domain::relay::{RelayRequest, RelayRequestContent, RelayType};
use ppaass_domain::session::Encryption;
use std::borrow::Cow;
use std::sync::Arc;
use tokio_util::bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
/// Relay request encoder is used by agent side after session init.
pub struct RelayRequestEncoder<F>
where
    F: EncryptionHolder,
{
    length_delimited_codec: LengthDelimitedCodec,
    encryption_holder: Arc<F>,
}
impl<F> RelayRequestEncoder<F>
where
    F: EncryptionHolder,
{
    pub fn new(encryption_holder: Arc<F>) -> Self {
        Self { encryption_holder, length_delimited_codec: LengthDelimitedCodec::new() }
    }
}
impl<F> Encoder<RelayRequest> for RelayRequestEncoder<F>
where
    F: EncryptionHolder,
{
    type Error = CodecError;
    fn encode(&mut self, item: RelayRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut relay_request_bytes = BytesMut::new();
        let session_token_bytes = item.session_token.as_bytes();
        relay_request_bytes.put_u64(session_token_bytes.len() as u64);
        relay_request_bytes.put_slice(session_token_bytes);
        match item.relay_type {
            RelayType::Tcp => {
                relay_request_bytes.put_u8(0);
            }
            RelayType::Udp => {
                relay_request_bytes.put_u8(1);
            }
        }
        let content_bytes = bincode::serialize(&item.content)?;
        let encryption = self.encryption_holder.get_encryption(&item.session_token)?.ok_or(CodecError::EncryptionNotExist(item.session_token.clone()))?;
        let content_bytes = match encryption.as_ref() {
            Encryption::Plain => {
                content_bytes
            }
            Encryption::Aes(aes_token) => {
                encrypt_with_aes(aes_token.as_slice(), &content_bytes)?
            }
        };
        relay_request_bytes.put_u64(content_bytes.len() as u64);
        relay_request_bytes.put(content_bytes.as_slice());
        self.length_delimited_codec.encode(relay_request_bytes.freeze(), dst)?;
        Ok(())
    }
}
/// Relay request decoder is used by proxy side after session init.
pub struct RelayRequestDecoder<F>
where
    F: EncryptionHolder,
{
    encryption_holder: Arc<F>,
    length_delimited_codec: LengthDelimitedCodec,
}
impl<F> RelayRequestDecoder<F>
where
    F: EncryptionHolder,
{
    pub fn new(encryption_holder: Arc<F>) -> Self {
        Self { encryption_holder, length_delimited_codec: LengthDelimitedCodec::new() }
    }
}
impl<F> Decoder for RelayRequestDecoder<F>
where
    F: EncryptionHolder,
{
    type Item = RelayRequest;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let relay_request_bytes = self.length_delimited_codec.decode(src)?;
        let mut relay_request_bytes = match relay_request_bytes {
            None => {
                return Ok(None);
            }
            Some(relay_request_bytes) => relay_request_bytes
        };
        let session_token_bytes_len = relay_request_bytes.get_u64();
        let session_token_bytes = relay_request_bytes.split_to(session_token_bytes_len as usize);
        let session_token = String::from_utf8_lossy(&session_token_bytes);
        let relay_type_byte = relay_request_bytes.get_u8();
        let relay_type = match relay_type_byte {
            0 => RelayType::Tcp,
            1 => RelayType::Udp,
            v => {
                return Err(CodecError::InvalidRelayTypeByte(v))
            }
        };
        let content_bytes_len = relay_request_bytes.get_u64();
        if relay_request_bytes.remaining() < content_bytes_len as usize {
            return Err(CodecError::NotEnoughRemainingBytes(content_bytes_len));
        }
        let encryption = self.encryption_holder.get_encryption(&session_token)?.ok_or(CodecError::EncryptionNotExist(session_token.clone().into()))?;
        let content_bytes = match encryption.as_ref() {
            Encryption::Plain => {
                Cow::Borrowed(relay_request_bytes.as_ref())
            }
            Encryption::Aes(aes_token) => {
                Cow::Owned(decrypt_with_aes(&aes_token, &relay_request_bytes)?)
            }
        };
        let content = bincode::deserialize::<RelayRequestContent>(&content_bytes)?;
        Ok(Some(RelayRequest {
            session_token: session_token.into(),
            relay_type,
            content,
        }))
    }
}