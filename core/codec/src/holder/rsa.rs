use crate::error::CodecError;
use ppaass_crypto::rsa::RsaCrypto;
use std::sync::Arc;
/// The rsa crypto fetcher,
/// each player have a rsa crypto
/// which can be fund from the storage
/// with user token
pub trait RsaCryptoHolder {
    /// Fetch the rsa crypto by user token
    fn get_rsa_crypto(
        &self,
        auth_token: impl AsRef<str>,
    ) -> Result<Option<Arc<RsaCrypto>>, CodecError>;
}
impl<T> RsaCryptoHolder for Arc<T>
where
    T: RsaCryptoHolder,
{
    fn get_rsa_crypto(
        &self,
        auth_token: impl AsRef<str>,
    ) -> Result<Option<Arc<RsaCrypto>>, CodecError> {
        RsaCryptoHolder::get_rsa_crypto(self.as_ref(), auth_token)
    }
}
impl<T> RsaCryptoHolder for &T
where
    T: RsaCryptoHolder,
{
    fn get_rsa_crypto(
        &self,
        auth_token: impl AsRef<str>,
    ) -> Result<Option<Arc<RsaCrypto>>, CodecError> {
        RsaCryptoHolder::get_rsa_crypto(*self, auth_token)
    }
}
impl<T> RsaCryptoHolder for &mut T
where
    T: RsaCryptoHolder,
{
    fn get_rsa_crypto(
        &self,
        auth_token: impl AsRef<str>,
    ) -> Result<Option<Arc<RsaCrypto>>, CodecError> {
        RsaCryptoHolder::get_rsa_crypto(*self, auth_token)
    }
}


