use thiserror::Error;

/// Errors returned by the WeChat Pay client.
#[derive(Debug, Error)]
pub enum WechatPayError {
    /// Raised when configuration is invalid.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    /// Raised when a request is missing required fields.
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    /// Raised when JSON serialization or deserialization fails.
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    /// Raised when XML parsing fails.
    #[error("xml parse error: {0}")]
    Xml(#[from] quick_xml::Error),
    /// Raised when HTTP transport fails.
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),
    /// Raised when reading a certificate fails.
    #[error("failed to read merchant certificate: {0}")]
    Io(#[from] std::io::Error),
    /// Raised when PKCS#12 identity loading fails.
    #[error("failed to load PKCS#12 identity: {0}")]
    Identity(String),
    /// Raised when a signature operation fails.
    #[error("signing error: {0}")]
    Signing(String),
    /// Raised when the API returns a transport-level failure.
    #[error("wechat pay transport failed: return_code={return_code}, return_msg={return_msg}")]
    Transport {
        return_code: String,
        return_msg: String,
        body: String,
    },
    /// Raised when the API returns a business failure.
    #[error(
        "wechat pay api failed: err_code={err_code:?}, err_code_des={err_code_des:?}, return_msg={return_msg:?}"
    )]
    Api {
        return_msg: Option<String>,
        err_code: Option<String>,
        err_code_des: Option<String>,
        body: String,
    },
    /// Raised when a response signature is invalid.
    #[error("invalid wechat pay signature")]
    InvalidSignature,
}
