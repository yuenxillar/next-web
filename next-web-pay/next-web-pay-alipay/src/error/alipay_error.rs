use thiserror::Error;

/// Errors returned by the Alipay client.
#[derive(Debug, Error)]
pub enum AlipayError {
    /// Raised when configuration is invalid.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    /// Raised when a parameter is missing.
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    /// Raised when request serialization fails.
    #[error("failed to serialize payload: {0}")]
    Serialize(#[from] serde_json::Error),
    /// Raised when HTTP transport fails.
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),
    /// Raised when the gateway returns a non-success HTTP status.
    #[error("alipay gateway returned http {status}: {body}")]
    HttpStatus { status: u16, body: String },
    /// Raised when the gateway response cannot be parsed.
    #[error("failed to parse alipay response for `{method}`: {body}")]
    ResponseFormat { method: String, body: String },
    /// Raised when RSA2 signing or verification fails.
    #[error("rsa2 error: {0}")]
    Signing(String),
    /// Raised when an API response contains a business error.
    #[error(
        "alipay api `{method}` failed with code={code}, msg={msg}, sub_code={sub_code:?}, sub_msg={sub_msg:?}"
    )]
    Api {
        method: String,
        code: String,
        msg: String,
        sub_code: Option<String>,
        sub_msg: Option<String>,
        body: String,
    },
    /// Raised when the notification signature is invalid.
    #[error("invalid alipay notification signature")]
    InvalidSignature,

    /// Raised when AES encryption fails.
    #[error("AES encryption failed: {0}")]
    EncryptionFailed(String),

    /// Raised when AES decryption fails.
    #[error("AES decryption failed: {0}")]
    DecryptionFailed(String),

    /// Raised when an unknown error occurs.
    #[error("Custom error: {0}")]
    Custom(String),
}

impl From<String> for AlipayError {
    fn from(value: String) -> Self {
        Self::Custom(value)
    }
}
