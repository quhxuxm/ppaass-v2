use crate::error::CodecError;
use ppaass_crypto::error::CryptoError;
use ppaass_crypto::rsa::RsaCryptoFetcher;
use ppaass_domain::session::{Encryption, SessionInitResponse};
use std::sync::Arc;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
/// Session init response encoder will be used by proxy side
pub struct SessionInitResponseEncoder<F>
where
    F: RsaCryptoFetcher,
{
    length_delimited_codec: LengthDelimitedCodec,
    rsa_crypto_fetcher: Arc<F>,
}
impl<F> SessionInitResponseEncoder<F>
where
    F: RsaCryptoFetcher,
{
    pub fn new(rsa_crypto_fetcher: Arc<F>) -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
            rsa_crypto_fetcher,
        }
    }
}
impl<F> Encoder<(String, SessionInitResponse)> for SessionInitResponseEncoder<F>
where
    F: RsaCryptoFetcher,
{
    type Error = CodecError;
    fn encode(&mut self, item: (String, SessionInitResponse), dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (auth_token, SessionInitResponse {
            proxy_encryption, session_token, status
        }) = item;
        let rsa_crypto = self.rsa_crypto_fetcher.fetch(&auth_token)?.ok_or(CryptoError::Rsa(format!("Rsa crypto not found: {auth_token}")))?;
        let proxy_encryption = match proxy_encryption {
            Encryption::Plain => proxy_encryption,
            Encryption::Aes(aes_token) => {
                Encryption::Aes(rsa_crypto.encrypt(&aes_token)?)
            }
        };
        let session_init_response = SessionInitResponse {
            proxy_encryption,
            session_token,
            status,
        };
        let session_init_response_bytes = bincode::serialize(&session_init_response)?;
        Ok(self.length_delimited_codec.encode(session_init_response_bytes.into(), dst)?)
    }
}
/// Session init response encoder will be used by agent side
pub struct SessionInitResponseDecoder<F>
where
    F: RsaCryptoFetcher,
{
    length_delimited_codec: LengthDelimitedCodec,
    rsa_crypto_fetcher: Arc<F>,
    auth_token: String,
}
impl<F> SessionInitResponseDecoder<F>
where
    F: RsaCryptoFetcher,
{
    pub fn new(auth_token: String, rsa_crypto_fetcher: Arc<F>) -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
            rsa_crypto_fetcher,
            auth_token,
        }
    }
}
impl<F> Decoder for SessionInitResponseDecoder<F>
where
    F: RsaCryptoFetcher,
{
    type Item = SessionInitResponse;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let session_init_response = self.length_delimited_codec.decode(src)?;
        match session_init_response {
            None => Ok(None),
            Some(session_init_response_bytes) => {
                let SessionInitResponse { proxy_encryption, session_token, status } = bincode::deserialize::<SessionInitResponse>(&session_init_response_bytes)?;
                let rsa_crypto = self.rsa_crypto_fetcher.fetch(&self.auth_token)?.ok_or(CryptoError::Rsa(format!("Rsa crypto not found: {}", self.auth_token)))?;
                let proxy_encryption = match proxy_encryption {
                    Encryption::Plain => proxy_encryption,
                    Encryption::Aes(aes_token) => {
                        Encryption::Aes(rsa_crypto.decrypt(&aes_token)?)
                    }
                };
                Ok(Some(SessionInitResponse {
                    proxy_encryption,
                    session_token,
                    status,
                }))
            }
        }
    }
}