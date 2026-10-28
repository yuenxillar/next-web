use crate::core::crypto::crypto_error::CryptoError;

pub trait CipherService {
    fn encrypt(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError>;
    fn decrypt(&self, encrypted: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError>;
}
