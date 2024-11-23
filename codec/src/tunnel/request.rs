use crate::error::CodecError;
use crate::RsaCryptoHolder;
use ppaass_crypto::error::CryptoError;
use ppaass_domain::tunnel::{Encryption, TunnelInitRequest};
use std::sync::Arc;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
/// Tunnel init request encoder will be used by agent side
pub struct TunnelInitRequestEncoder<F>
where
    F: RsaCryptoHolder,
{
    length_delimited_codec: LengthDelimitedCodec,
    rsa_crypto_fetcher: Arc<F>,
}
impl<F> TunnelInitRequestEncoder<F>
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
impl<F> Encoder<TunnelInitRequest> for TunnelInitRequestEncoder<F>
where
    F: RsaCryptoHolder,
{
    type Error = CodecError;
    fn encode(&mut self, item: TunnelInitRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let TunnelInitRequest {
            agent_encryption, auth_token, dst_address, tunnel_type,
        } = item;
        let rsa_crypto = self.rsa_crypto_fetcher.get_rsa_crypto(&auth_token)?.ok_or(CryptoError::Rsa(format!("Rsa crypto not found: {auth_token}")))?;
        let agent_encryption = match agent_encryption {
            Encryption::Plain => agent_encryption,
            Encryption::Aes(aes_token) => {
                Encryption::Aes(rsa_crypto.encrypt(&aes_token)?)
            }
        };
        let tunnel_init_request = TunnelInitRequest {
            agent_encryption,
            auth_token,
            dst_address,
            tunnel_type,
        };
        let tunnel_init_request_bytes = bincode::serialize(&tunnel_init_request)?;
        Ok(self.length_delimited_codec.encode(tunnel_init_request_bytes.into(), dst)?)
    }
}
/// Tunnel init request decoder will be used by proxy side
pub struct TunnelInitRequestDecoder<F>
where
    F: RsaCryptoHolder,
{
    length_delimited_codec: LengthDelimitedCodec,
    rsa_crypto_fetcher: Arc<F>,
}
impl<F> TunnelInitRequestDecoder<F>
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
impl<F> Decoder for TunnelInitRequestDecoder<F>
where
    F: RsaCryptoHolder,
{
    type Item = TunnelInitRequest;
    type Error = CodecError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let tunnel_init_request = self.length_delimited_codec.decode(src)?;
        match tunnel_init_request {
            None => Ok(None),
            Some(tunnel_init_request_bytes) => {
                let TunnelInitRequest { agent_encryption, auth_token, dst_address, tunnel_type, } = bincode::deserialize::<TunnelInitRequest>(&tunnel_init_request_bytes)?;
                let rsa_crypto = self.rsa_crypto_fetcher.get_rsa_crypto(&auth_token)?.ok_or(CryptoError::Rsa(format!("Rsa crypto not found: {auth_token}")))?;
                let agent_encryption = match agent_encryption {
                    Encryption::Plain => agent_encryption,
                    Encryption::Aes(aes_token) => {
                        Encryption::Aes(rsa_crypto.decrypt(&aes_token)?)
                    }
                };
                Ok(Some(TunnelInitRequest {
                    agent_encryption,
                    auth_token,
                    dst_address,
                    tunnel_type,
                }))
            }
        }
    }
}