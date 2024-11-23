use crate::crypto::AgentRsaCryptoHolder;
use crate::error::AgentError;
use bytes::BytesMut;
use ppaass_codec::{TunnelInitRequestEncoder, TunnelInitResponseDecoder};
use ppaass_domain::tunnel::{TunnelInitRequest, TunnelInitResponse};
use std::sync::Arc;
use tokio_util::codec::{Decoder, Encoder};
pub struct ProxyConnectionCodec {
    tunnel_init_response_decoder: TunnelInitResponseDecoder<AgentRsaCryptoHolder>,
    tunnel_init_request_encoder: TunnelInitRequestEncoder<AgentRsaCryptoHolder>,
}
impl ProxyConnectionCodec {
    pub fn new(auth_token: String, rsa_crypto_holder: Arc<AgentRsaCryptoHolder>) -> Self {
        Self {
            tunnel_init_response_decoder: TunnelInitResponseDecoder::new(
                auth_token,
                rsa_crypto_holder.clone(),
            ),
            tunnel_init_request_encoder: TunnelInitRequestEncoder::new(rsa_crypto_holder),
        }
    }
}
impl Encoder<TunnelInitRequest> for ProxyConnectionCodec {
    type Error = AgentError;
    fn encode(&mut self, item: TunnelInitRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.tunnel_init_request_encoder.encode(item, dst)?;
        Ok(())
    }
}
impl Decoder for ProxyConnectionCodec {
    type Item = TunnelInitResponse;
    type Error = AgentError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.tunnel_init_response_decoder.decode(src)?)
    }
}
