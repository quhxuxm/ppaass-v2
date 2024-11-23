use crate::error::CodecError;
use crate::RsaCryptoHolder;
use ppaass_crypto::error::CryptoError;
use ppaass_domain::tunnel::{Encryption, TunnelInitResponse};
use std::sync::Arc;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
/// Tunnel init response encoder will be used by proxy side
pub struct TunnelInitResponseEncoder<F>
where
    F: RsaCryptoHolder,
{
    length_delimited_codec: LengthDelimitedCodec,
    rsa_crypto_fetcher: Arc<F>,
}
impl<F> TunnelInitResponseEncoder<F>
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
impl<F> Encoder<(String, TunnelInitResponse)> for TunnelInitResponseEncoder<F>
where
    F: RsaCryptoHolder,
{
    type Error = CodecError;
    fn encode(&mut self, item: (String, TunnelInitResponse), dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (auth_token, TunnelInitResponse {
            proxy_encryption
        }) = item;
        let rsa_crypto = self.rsa_crypto_fetcher.get_rsa_crypto(&auth_token)?.ok_or(CryptoError::Rsa(format!("Rsa crypto not found: {auth_token}")))?;
        let proxy_encryption = match proxy_encryption {
            Encryption::Plain => proxy_encryption,
            Encryption::Aes(aes_token) => {
                Encryption::Aes(rsa_crypto.encrypt(&aes_token)?)
            }
        };
        let tunnel_init_response = TunnelInitResponse {
            proxy_encryption,
        };
        let tunnel_init_response_bytes = bincode::serialize(&tunnel_init_response)?;
        Ok(self.length_delimited_codec.encode(tunnel_init_response_bytes.into(), dst)?)
    }
}
/// Tunnel init response encoder will be used by agent side
pub struct TunnelInitResponseDecoder<F>
where
    F: RsaCryptoHolder,
{
    length_delimited_codec: LengthDelimitedCodec,
    rsa_crypto_fetcher: Arc<F>,
    auth_token: String,
}
impl<F> TunnelInitResponseDecoder<F>
where
    F: RsaCryptoHolder,
{
    pub fn new(auth_token: String, rsa_crypto_fetcher: Arc<F>) -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
            rsa_crypto_fetcher,
            auth_token,
        }
    }
}
impl<F> Decoder for TunnelInitResponseDecoder<F>
where
    F: RsaCryptoHolder,
{
    type Item = TunnelInitResponse;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let tunnel_init_response = self.length_delimited_codec.decode(src)?;
        match tunnel_init_response {
            None => Ok(None),
            Some(tunnel_init_response_bytes) => {
                let TunnelInitResponse { proxy_encryption } = bincode::deserialize::<TunnelInitResponse>(&tunnel_init_response_bytes)?;
                let rsa_crypto = self.rsa_crypto_fetcher.get_rsa_crypto(&self.auth_token)?.ok_or(CryptoError::Rsa(format!("Rsa crypto not found: {}", self.auth_token)))?;
                let proxy_encryption = match proxy_encryption {
                    Encryption::Plain => proxy_encryption,
                    Encryption::Aes(aes_token) => {
                        Encryption::Aes(rsa_crypto.decrypt(&aes_token)?)
                    }
                };
                Ok(Some(TunnelInitResponse {
                    proxy_encryption,
                }))
            }
        }
    }
}