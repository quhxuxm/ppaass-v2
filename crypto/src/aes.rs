use crate::error::CryptoError;
use crate::random_32_bytes;
use aes::Aes256;
use bytes::Bytes;
use cipher::block_padding::Pkcs7;
use cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
pub fn generate_aes_encryption_token() -> Bytes {
    random_32_bytes()
}

pub fn encrypt_with_aes(encryption_token: &Bytes, target: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let aes_encryptor = Aes256::new(encryption_token[..].into());
    let result = aes_encryptor.encrypt_padded_vec::<Pkcs7>(target);
    Ok(result)
}

pub fn decrypt_with_aes(encryption_token: &Bytes, target: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let aes_decryptor = Aes256::new(encryption_token[..].into());
    let result = aes_decryptor
        .decrypt_padded_vec::<Pkcs7>(target)
        .map_err(|e| CryptoError::Aes(format!("Fail to decrypt with aes block: {e:?}")))?;
    Ok(result)
}

#[test]
fn test() -> Result<(), CryptoError> {
    let encryption_token = generate_aes_encryption_token();
    let target = "hello world! this is my plaintext.".as_bytes().to_vec();
    let encrypt_result = encrypt_with_aes(&encryption_token, &target)?;
    println!(
        "Encrypt result: [{:?}]",
        String::from_utf8_lossy(&encrypt_result)
    );
    let decrypted_result = decrypt_with_aes(&encryption_token, &encrypt_result)?;
    println!(
        "Decrypted result: [{:?}]",
        String::from_utf8_lossy(&decrypted_result)
    );
    Ok(())
}
