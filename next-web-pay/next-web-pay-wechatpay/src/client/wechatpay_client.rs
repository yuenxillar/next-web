use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::sha2::Sha256;
use rsa::signature::SignatureEncoding;
use rsa::signature::SignerMut;
use serde::Serialize;

use crate::WechatPayResult;
use crate::config::WechatPayConfig;
use crate::error::WechatPayError;
use crate::sign::generate_nonce_str;

/// Result of V3 request signing.
#[derive(Debug, Clone)]
pub struct V3SignedRequest {
    /// The JSON request body.
    pub body: String,
    /// The value for the Authorization header.
    pub authorization: String,
}

/// WeChat Pay V2 client.
#[derive(Debug, Clone)]
pub struct WechatPayClient {
    config: WechatPayConfig,
    _client: reqwest::Client,
}

impl WechatPayClient {
    /// Creates a client with a default HTTP client.
    pub fn new(config: WechatPayConfig) -> Self {
        Self {
            config,
            _client: reqwest::Client::new(),
        }
    }

    /// Creates a client with a custom HTTP client.
    pub fn with_http_client(config: WechatPayConfig, http_client: reqwest::Client) -> Self {
        Self {
            config,
            _client: http_client,
        }
    }

    /// Returns the http client.
    pub fn client(&self) -> &reqwest::Client {
        &self._client
    }

    /// Returns the immutable client config.
    pub fn config(&self) -> &WechatPayConfig {
        &self.config
    }

    /// Signs a V3 JSON request, returning the body and Authorization header.
    ///
    /// The `path` should be the URL path (e.g. "/v3/global/micropay/transactions/pay").
    pub fn signed_params<T: Serialize>(
        &self,
        method: &str,
        req_body: &T,
        path: &str,
    ) -> WechatPayResult<V3SignedRequest> {
        let serial_no = self.config.merchant_serial_no.as_ref().ok_or_else(|| {
            WechatPayError::InvalidConfig("merchant_serial_no is empty".to_string())
        })?;
        let private_key_pem = self.config.merchant_private_key.as_ref().ok_or_else(|| {
            WechatPayError::InvalidConfig("merchant_private_key is empty".to_string())
        })?;

        let body = serde_json::to_string(req_body)?;
        let timestamp = unix_timestamp();
        let nonce_str = generate_nonce_str();

        // 构造签名串
        //
        // HTTP请求方法\n
        // URL\n
        // 请求时间戳\n
        // 请求随机串\n
        // 请求报文主体\n
        let message = format!(
            "{method}\n{path}\n{timestamp}\n{nonce_str}\n{}\n",
            if method.to_ascii_uppercase() == "GET" {
                ""
            } else {
                &body
            }
        );

        let signature = sign_v3_message(&message, private_key_pem)?;

        let authorization = format!(
            "WECHATPAY2-SHA256-RSA2048 mchid=\"{}\",serial_no=\"{serial_no}\",nonce_str=\"{nonce_str}\",timestamp=\"{timestamp}\",signature=\"{signature}\"",
            self.config.mch_id,
        );

        Ok(V3SignedRequest {
            body,
            authorization,
        })
    }
    fn validate_config(&self) -> WechatPayResult<()> {
        if self.config.app_id.trim().is_empty() {
            return Err(WechatPayError::InvalidConfig("app_id is empty".to_string()));
        }
        if self.config.mch_id.trim().is_empty() {
            return Err(WechatPayError::InvalidConfig("mch_id is empty".to_string()));
        }
        if self.config.api_key.trim().is_empty() {
            return Err(WechatPayError::InvalidConfig(
                "api_key is empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[inline]
fn unix_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

#[inline]
fn sign_v3_message(message: &str, private_key_pem: &str) -> WechatPayResult<String> {
    let private_key = rsa::RsaPrivateKey::from_pkcs8_pem(private_key_pem)
        .or_else(|_| rsa::RsaPrivateKey::from_pkcs1_pem(private_key_pem))
        .map_err(|e| WechatPayError::Signing(format!("invalid private key: {e}")))?;
    let mut signing_key = rsa::pkcs1v15::SigningKey::<Sha256>::new(private_key);
    let sig: rsa::pkcs1v15::Signature = signing_key
        .try_sign(message.as_bytes())
        .map_err(|e| WechatPayError::Signing(format!("signing failed: {e}")))?;
    Ok(STANDARD.encode(sig.to_bytes()))
}
