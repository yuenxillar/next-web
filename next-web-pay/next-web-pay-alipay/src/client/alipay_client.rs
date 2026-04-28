use std::collections::BTreeMap;

use chrono::Local;
use serde::Serialize;

use crate::config::AlipayConfig;

use crate::sign::{
    build_notify_sign_content, build_sign_content, cert_sn_from_path, public_key_from_cert_path,
    root_cert_sn_from_path, sign_with_rsa2, verify_with_rsa2,
};

/// High-level Alipay OpenAPI client.
#[derive(Debug, Clone)]
pub struct AlipayClient {
    config: AlipayConfig,
    certificate_info: Option<AlipayCertificateInfo>,
    _client: reqwest::Client,
}

#[derive(Debug, Clone)]
struct AlipayCertificateInfo {
    app_cert_sn: Option<String>,
    alipay_root_cert_sn: Option<String>,
    alipay_public_key: Option<String>,
}

impl AlipayClient {
    /// Creates a client with a default `reqwest::Client`.
    pub fn new(config: AlipayConfig) -> Self {
        let certificate_info = load_certificate_info(&config);
        Self {
            config,
            certificate_info,
            _client: reqwest::Client::new(),
        }
    }

    /// Creates a client with a custom `reqwest::Client`.
    pub fn with_http_client(config: AlipayConfig, _client: reqwest::Client) -> Self {
        let certificate_info = load_certificate_info(&config);
        Self {
            config,
            certificate_info,
            _client,
        }
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
        let borrowed = params
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect::<BTreeMap<&str, &str>>();
        let content = build_notify_sign_content(&borrowed);
        self.verify_signature(&content, signature)
    }

    /// Verifies content with the Alipay public certificate when configured,
    /// otherwise falls back to the configured Alipay public key.
    pub fn verify_signature(&self, content: &str, signature: &str) -> Result<bool, String> {
        if let Some(certificate_info) = self.certificate_info() {
            if let Some(alipay_public_key) = certificate_info.alipay_public_key.as_deref() {
                return verify_with_rsa2(content, signature, alipay_public_key);
            }
        }

        verify_with_rsa2(content, signature, self.config.alipay_public_key())
    }

    pub fn signed_params<T>(
        &self,
        method: &str,
        biz_content: &T,
        extra_params: Option<BTreeMap<&'static str, String>>,
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

        if let Some(certificate_info) = self.certificate_info() {
            if let Some(app_cert_sn) = certificate_info.app_cert_sn.as_ref() {
                params.insert("app_cert_sn", app_cert_sn.clone());
            }
            if let Some(alipay_root_cert_sn) = certificate_info.alipay_root_cert_sn.as_ref() {
                params.insert("alipay_root_cert_sn", alipay_root_cert_sn.clone());
            }
        }

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

        extra_params.map(|ext| params.extend(ext));

        let sign = sign_with_rsa2(
            build_sign_content(& params).as_str(),
            self.config.merchant_private_key(),
        )?;
        params.insert("sign", sign);

        Ok(params)
    }

    fn certificate_info(&self) -> Option<&AlipayCertificateInfo> {
        self.certificate_info.as_ref()
    }
}

fn load_certificate_info(config: &AlipayConfig) -> Option<AlipayCertificateInfo> {
    if config.merchant_cert_path().is_none()
        && config.alipay_root_cert_path().is_none()
        && config.alipay_cert_path().is_none()
    {
        return None;
    }

    Some((|| {
        let app_cert_sn = config
            .merchant_cert_path()
            .map(cert_sn_from_path)
            .transpose()
            .expect("Failed to get merchant certificate SN");
        let alipay_root_cert_sn = config
            .alipay_root_cert_path()
            .map(root_cert_sn_from_path)
            .transpose()
            .expect("Failed to get alipay root certificate SN");
        let alipay_public_key = config
            .alipay_cert_path()
            .map(public_key_from_cert_path)
            .transpose()
            .expect("Failed to get alipay public key from certificate");

        AlipayCertificateInfo {
            app_cert_sn,
            alipay_root_cert_sn,
            alipay_public_key,
        }
    })())
}
