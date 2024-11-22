use crate::error::CodecError;
use crate::EncryptionHolder;
use ppaass_crypto::aes::{decrypt_with_aes, encrypt_with_aes};
use ppaass_domain::relay::{RelayResponse, RelayResponseContent, RelayResponseStatus, RelayType};
use ppaass_domain::session::Encryption;
use std::borrow::Cow;
use std::sync::Arc;
use tokio_util::bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
/// Relay response encoder is used by proxy side after session init.
pub struct RelayResponseEncoder<T>
where
    T: EncryptionHolder,
{
    length_delimited_codec: LengthDelimitedCodec,
    encryption_holder: Arc<T>,
}
impl<T> RelayResponseEncoder<T>
where
    T: EncryptionHolder,
{
    pub fn new(encryption_holder: Arc<T>) -> Self {
        Self { length_delimited_codec: LengthDelimitedCodec::new(), encryption_holder }
    }
}
impl<T> Encoder<RelayResponse> for RelayResponseEncoder<T>
where
    T: EncryptionHolder,
{
    type Error = CodecError;
    fn encode(&mut self, item: RelayResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut relay_response_bytes = BytesMut::new();
        let session_token_bytes = item.session_token.as_bytes();
        relay_response_bytes.put_u64(session_token_bytes.len() as u64);
        relay_response_bytes.put_slice(session_token_bytes);
        match item.status {
            RelayResponseStatus::Success => {
                relay_response_bytes.put_u8(0);
            }
            RelayResponseStatus::SessionNotFound => {
                relay_response_bytes.put_u8(1);
            }
            RelayResponseStatus::DestinationUnreachable => {
                relay_response_bytes.put_u8(2);
            }
        }
        match item.relay_type {
            RelayType::Tcp => {
                relay_response_bytes.put_u8(0);
            }
            RelayType::Udp => {
                relay_response_bytes.put_u8(1);
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
        relay_response_bytes.put_u64(content_bytes.len() as u64);
        relay_response_bytes.put(content_bytes.as_slice());
        self.length_delimited_codec.encode(relay_response_bytes.freeze(), dst)?;
        Ok(())
    }
}
/// Relay response decoder is used by agent side after session init.
pub struct RelayResponseDecoder<T>
where
    T: EncryptionHolder,
{
    length_delimited_codec: LengthDelimitedCodec,
    encryption_holder: Arc<T>,
}
impl<T> RelayResponseDecoder<T>
where
    T: EncryptionHolder,
{
    pub fn new(encryption_holder: Arc<T>) -> Self {
        Self { length_delimited_codec: LengthDelimitedCodec::new(), encryption_holder }
    }
}
impl<T> Decoder for RelayResponseDecoder<T>
where
    T: EncryptionHolder,
{
    type Item = RelayResponse;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let relay_response_bytes = self.length_delimited_codec.decode(src)?;
        let mut relay_response_bytes = match relay_response_bytes {
            None => {
                return Ok(None);
            }
            Some(relay_response_bytes) => relay_response_bytes
        };
        let session_token_bytes_len = relay_response_bytes.get_u64();
        let session_token_bytes = relay_response_bytes.split_to(session_token_bytes_len as usize);
        let session_token = String::from_utf8_lossy(&session_token_bytes);
        let status_byte = relay_response_bytes.get_u8();
        let status = match status_byte {
            0 => RelayResponseStatus::Success,
            1 => RelayResponseStatus::SessionNotFound,
            2 => RelayResponseStatus::DestinationUnreachable,
            v => {
                return Err(CodecError::InvalidRelayResponseStatusByte(v))
            }
        };
        let relay_type_byte = relay_response_bytes.get_u8();
        let relay_type = match relay_type_byte {
            0 => RelayType::Tcp,
            1 => RelayType::Udp,
            v => {
                return Err(CodecError::InvalidRelayTypeByte(v))
            }
        };
        let content_bytes_len = relay_response_bytes.get_u64();
        if relay_response_bytes.remaining() < content_bytes_len as usize {
            return Err(CodecError::NotEnoughRemainingBytes(content_bytes_len));
        }
        let encryption = self.encryption_holder.get_encryption(&session_token)?.ok_or(CodecError::EncryptionNotExist(session_token.clone().into()))?;
        let content_bytes = match &encryption.as_ref() {
            Encryption::Plain => {
                Cow::Borrowed(relay_response_bytes.as_ref())
            }
            Encryption::Aes(aes_token) => {
                Cow::Owned(decrypt_with_aes(&aes_token, &relay_response_bytes)?)
            }
        };
        let content = bincode::deserialize::<RelayResponseContent>(&content_bytes)?;
        Ok(Some(RelayResponse {
            session_token: session_token.into(),
            relay_type,
            status,
            content,
        }))
    }
}
