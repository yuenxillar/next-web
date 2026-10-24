use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub enum CryptoError {
    EncryptionError(String),
    DecryptionError(String),
    InvalidKey,
}

impl Error for CryptoError {}

impl Display for CryptoError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            CryptoError::EncryptionError(msg) => write!(f, "Encryption error: {}", msg),
            CryptoError::DecryptionError(msg) => write!(f, "Decryption error: {}", msg),
            CryptoError::InvalidKey => write!(f, "Invalid key"),
        }
    }
}
