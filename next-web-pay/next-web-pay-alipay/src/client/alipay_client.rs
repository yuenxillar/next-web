use std::collections::BTreeMap;
use std::fmt::format;

use chrono::Local;
use serde::Serialize;

use crate::config::AlipayConfig;

use crate::notify::PaymentNotification;
use crate::sign::{build_sign_content, sign_with_rsa2, verify_with_rsa2};

/// High-level Alipay OpenAPI client.
#[derive(Debug, Clone)]
pub struct AlipayClient {
    config: AlipayConfig,
    _client: reqwest::Client,
}

impl AlipayClient {
    /// Creates a client with a default `reqwest::Client`.
    pub fn new(config: AlipayConfig) -> Self {
        Self {
            config,
            _client: reqwest::Client::new(),
        }
    }

    /// Creates a client with a custom `reqwest::Client`.
    pub fn with_http_client(config: AlipayConfig, _client: reqwest::Client) -> Self {
        Self { config, _client }
    }

    /// Returns the client configuration.
    pub fn config(&self) -> &AlipayConfig {
        &self.config
    }

    pub fn client(&self) -> &reqwest::Client {
        &self._client
    }

    /// Verifies an asynchronous notification signature.
    pub fn verify_notification_signature(
        &self,
        params: &BTreeMap<&'static str, String>,
    ) -> Result<bool, String> {
        let signature = params.get("sign").ok_or("Missing signature".to_string())?;
        let content = build_sign_content(params);
        verify_with_rsa2(&content, signature, self.config.alipay_public_key())
    }

    pub fn signed_params<T>(
        &self,
        method: &str,
        biz_content: &T,
    ) -> Result<BTreeMap<&'static str, String>, String>
    where
        T: Serialize + ?Sized,
    {
        if self.config.app_id().trim().is_empty() {
            return Err("app_id is empty".to_string());
        }
        if self.config.merchant_private_key().trim().is_empty() {
            return Err("merchant_private_key is empty".to_string());
        }

        let mut params = BTreeMap::new();
        params.insert("app_id", self.config.app_id().into());
        params.insert("method", method.into());
        params.insert("format", "JSON".into());
        params.insert("charset", self.config.charset().into());
        params.insert("sign_type", self.config.sign_type().into());
        params.insert(
            "timestamp",
            Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        );
        params.insert("version", self.config.version().into());

        if let Some(notify_url) = self.config.notify_url() {
            params.insert("notify_url", notify_url.into());
        }
        if let Some(app_auth_token) = self.config.app_auth_token() {
            params.insert("app_auth_token", app_auth_token.into());
        }

        params.insert(
            "biz_content",
            serde_json::to_string(biz_content)
                .map_err(|e| format!("Failed to serialize biz_content to JSON: {}", e))?,
        );

        let sign = sign_with_rsa2(
            &build_sign_content(&params),
            self.config.merchant_private_key(),
        )?;
        params.insert("sign", sign);

        Ok(params)
    }
}