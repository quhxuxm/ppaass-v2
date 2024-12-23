use crate::config::Config;
use ppaass_codec::error::CodecError;
use ppaass_codec::RsaCryptoHolder;
use ppaass_crypto::error::CryptoError;
use ppaass_crypto::rsa::RsaCrypto;
use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::path::Path;
use std::sync::Arc;
use tracing::error;
pub struct AgentRsaCryptoHolder {
    cache: Arc<HashMap<String, Arc<RsaCrypto>>>,
}

impl AgentRsaCryptoHolder {
    pub fn new(config: Arc<Config>) -> Result<Self, CryptoError> {
        let mut cache = HashMap::new();
        let rsa_dir_path = config.rsa_dir();
        let rsa_dir = read_dir(rsa_dir_path)?;
        rsa_dir.for_each(|entry| {
            let Ok(entry) = entry else {
                error!("fail to read {rsa_dir_path:?} directory");
                return;
            };
            let user_token = entry.file_name();
            let user_token = user_token.to_str();
            let Some(user_token) = user_token else {
                error!(
                    "Fail to read {rsa_dir_path:?}{:?} directory because of user token not exist",
                    entry.file_name()
                );
                return;
            };
            let public_key_path = rsa_dir_path.join(user_token).join("ProxyPublicKey.pem");
            let Ok(public_key_file) = File::open(&public_key_path) else {
                error!("Fail to read public key file: {public_key_path:?}.");
                return;
            };
            let private_key_path = rsa_dir_path.join(user_token).join("AgentPrivateKey.pem");
            let private_key_path = Path::new(Path::new(&private_key_path));
            let Ok(private_key_file) = File::open(private_key_path) else {
                error!("Fail to read private key file :{private_key_path:?}.");
                return;
            };
            let Ok(rsa_crypto) = RsaCrypto::new(public_key_file, private_key_file) else {
                error!("Fail to create rsa crypto for user: {user_token}.");
                return;
            };
            cache.insert(user_token.to_string(), Arc::new(rsa_crypto));
        });
        Ok(Self {
            cache: Arc::new(cache),
        })
    }
}

impl RsaCryptoHolder for AgentRsaCryptoHolder {
    fn get_rsa_crypto(
        &self,
        auth_token: impl AsRef<str>,
    ) -> Result<Option<Arc<RsaCrypto>>, CodecError> {
        match self.cache.get(auth_token.as_ref()) {
            None => Ok(None),
            Some(val) => Ok(Some(val.clone())),
        }
    }
}
