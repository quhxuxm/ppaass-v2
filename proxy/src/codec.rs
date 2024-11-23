use crate::crypto::ProxyRsaCryptoHolder;
use crate::error::ProxyError;
use bytes::BytesMut;
use ppaass_codec::{TunnelInitRequestDecoder, TunnelInitResponseEncoder};
use ppaass_domain::tunnel::{TunnelInitRequest, TunnelInitResponse};
use std::sync::Arc;
use tokio_util::codec::{Decoder, Encoder};
pub struct AgentConnectionCodec
{
    tunnel_init_request_decoder: TunnelInitRequestDecoder<ProxyRsaCryptoHolder>,
    tunnel_init_response_encoder: TunnelInitResponseEncoder<ProxyRsaCryptoHolder>,
}
impl AgentConnectionCodec
{
    pub fn new(rsa_crypto_holder: Arc<ProxyRsaCryptoHolder>) -> Self {
        Self {
            tunnel_init_request_decoder: TunnelInitRequestDecoder::new(rsa_crypto_holder.clone()),
            tunnel_init_response_encoder: TunnelInitResponseEncoder::new(rsa_crypto_holder),
        }
    }
}
impl Encoder<(String, TunnelInitResponse)> for AgentConnectionCodec
{
    type Error = ProxyError;
    fn encode(&mut self, item: (String, TunnelInitResponse), dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.tunnel_init_response_encoder.encode(item, dst)?;
        Ok(())
    }
}
impl Decoder for AgentConnectionCodec
{
    type Item = TunnelInitRequest;
    type Error = ProxyError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.tunnel_init_request_decoder.decode(src)?)
    }
}