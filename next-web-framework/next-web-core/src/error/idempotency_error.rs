use thiserror::Error;

/// 幂等性错误类型
#[derive(Error, Debug)]
pub enum IdempotencyError {
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    
    #[error("Key already exists")]
    KeyExists,
}