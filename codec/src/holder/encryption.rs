use crate::error::CodecError;
use ppaass_domain::tunnel::Encryption;
use std::sync::Arc;
pub trait EncryptionHolder {
    fn get_encryption(
        &self,
        encryption_key: impl AsRef<str>,
    ) -> Result<Option<Arc<Encryption>>, CodecError>;
}
impl<T> EncryptionHolder for Arc<T>
where
    T: EncryptionHolder,
{
    fn get_encryption(
        &self,
        encryption_key: impl AsRef<str>,
    ) -> Result<Option<Arc<Encryption>>, CodecError> {
        EncryptionHolder::get_encryption(self.as_ref(), encryption_key)
    }
}
impl<T> EncryptionHolder for &T
where
    T: EncryptionHolder,
{
    fn get_encryption(
        &self,
        encryption_key: impl AsRef<str>,
    ) -> Result<Option<Arc<Encryption>>, CodecError> {
        EncryptionHolder::get_encryption(*self, encryption_key)
    }
}
impl<T> EncryptionHolder for &mut T
where
    T: EncryptionHolder,
{
    fn get_encryption(
        &self,
        encryption_key: impl AsRef<str>,
    ) -> Result<Option<Arc<Encryption>>, CodecError> {
        EncryptionHolder::get_encryption(*self, encryption_key)
    }
}
