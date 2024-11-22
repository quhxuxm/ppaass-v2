use crate::crypto::ProxyRsaCryptoFetcher;
use crate::error::ProxyError;
use bytes::BytesMut;
use ppaass_codec::{SessionInitRequestDecoder, SessionInitResponseEncoder};
use ppaass_domain::session::{SessionInitRequest, SessionInitResponse};
use std::sync::Arc;
use tokio_util::codec::{Decoder, Encoder};
pub struct SessionInitCodec {
    session_init_request_decoder: SessionInitRequestDecoder<ProxyRsaCryptoFetcher>,
    session_init_response_encoder: SessionInitResponseEncoder<ProxyRsaCryptoFetcher>,
}
impl SessionInitCodec {
    pub fn new(rsa_crypto_fetcher: Arc<ProxyRsaCryptoFetcher>) -> Self {
        Self {
            session_init_request_decoder: SessionInitRequestDecoder::new(rsa_crypto_fetcher.clone()),
            session_init_response_encoder: SessionInitResponseEncoder::new(rsa_crypto_fetcher),
        }
    }
}
impl Encoder<(String, SessionInitResponse)> for SessionInitCodec {
    type Error = ProxyError;
    fn encode(&mut self, item: (String, SessionInitResponse), dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(self.session_init_response_encoder.encode(item, dst)?)
    }
}
impl Decoder for SessionInitCodec {
    type Item = SessionInitRequest;
    type Error = ProxyError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.session_init_request_decoder.decode(src)?)
    }
}