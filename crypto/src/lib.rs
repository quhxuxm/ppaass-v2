use rand::random;
pub mod aes;
pub mod error;
pub mod rsa;
pub fn random_32_bytes() -> Vec<u8> {
    let random_32_bytes = random::<[u8; 32]>();
    random_32_bytes.to_vec()
}
