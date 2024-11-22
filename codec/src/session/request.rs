use crate::error::CodecError;
use crate::RsaCryptoHolder;
use ppaass_crypto::error::CryptoError;
use ppaass_domain::session::{Encryption, SessionInitRequest};
use std::sync::Arc;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
/// Session init request encoder will be used by agent side
pub struct SessionInitRequestEncoder<F>
where
    F: RsaCryptoHolder,
{
    length_delimited_codec: LengthDelimitedCodec,
    rsa_crypto_fetcher: Arc<F>,
}
impl<F> SessionInitRequestEncoder<F>
where
    F: RsaCryptoHolder,
{
    pub fn new(rsa_crypto_fetcher: Arc<F>) -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
            rsa_crypto_fetcher,
        }
    }
}
impl<F> Encoder<SessionInitRequest> for SessionInitRequestEncoder<F>
where
    F: RsaCryptoHolder,
{
    type Error = CodecError;
    fn encode(&mut self, item: SessionInitRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let SessionInitRequest {
            agent_encryption, auth_token
        } = item;
        let rsa_crypto = self.rsa_crypto_fetcher.get_rsa_crypto(&auth_token)?.ok_or(CryptoError::Rsa(format!("Rsa crypto not found: {auth_token}")))?;
        let agent_encryption = match agent_encryption {
            Encryption::Plain => agent_encryption,
            Encryption::Aes(aes_token) => {
                Encryption::Aes(rsa_crypto.encrypt(&aes_token)?)
            }
        };
        let session_init_request = SessionInitRequest {
            agent_encryption,
            auth_token,
        };
        let session_init_request_bytes = bincode::serialize(&session_init_request)?;
        Ok(self.length_delimited_codec.encode(session_init_request_bytes.into(), dst)?)
    }
}
/// Session init request decoder will be used by proxy side
pub struct SessionInitRequestDecoder<F>
where
    F: RsaCryptoHolder,
{
    length_delimited_codec: LengthDelimitedCodec,
    rsa_crypto_fetcher: Arc<F>,
}
impl<F> SessionInitRequestDecoder<F>
where
    F: RsaCryptoHolder,
{
    pub fn new(rsa_crypto_fetcher: Arc<F>) -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
            rsa_crypto_fetcher,
        }
    }
}
impl<F> Decoder for SessionInitRequestDecoder<F>
where
    F: RsaCryptoHolder,
{
    type Item = SessionInitRequest;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let session_init_request = self.length_delimited_codec.decode(src)?;
        match session_init_request {
            None => Ok(None),
            Some(session_init_request_bytes) => {
                let SessionInitRequest { agent_encryption, auth_token } = bincode::deserialize::<SessionInitRequest>(&session_init_request_bytes)?;
                let rsa_crypto = self.rsa_crypto_fetcher.get_rsa_crypto(&auth_token)?.ok_or(CryptoError::Rsa(format!("Rsa crypto not found: {auth_token}")))?;
                let agent_encryption = match agent_encryption {
                    Encryption::Plain => agent_encryption,
                    Encryption::Aes(aes_token) => {
                        Encryption::Aes(rsa_crypto.decrypt(&aes_token)?)
                    }
                };
                Ok(Some(SessionInitRequest {
                    agent_encryption,
                    auth_token,
                }))
            }
        }
    }
}