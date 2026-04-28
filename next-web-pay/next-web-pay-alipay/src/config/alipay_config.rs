use std::fmt::Debug;

/// Alipay gateway configuration.
#[derive(Clone)]
pub struct AlipayConfig {
    /// Alipay app id.
    app_id: String,

    /// Merchant RSA private key in PEM or base64 form.
    merchant_private_key: String,

    /// Alipay RSA public key in PEM or base64 form.
    alipay_public_key: String,

    /// Gateway endpoint.
    gateway_url: String,

    /// Request charset.
    charset: String,

    /// Response format.
    format: String,

    /// Signature type.
    sign_type: String,

    /// OpenAPI version.
    version: String,

    /// Merchant certificate path.
    merchant_cert_path: Option<String>,

    /// Alipay certificate path.
    alipay_cert_path: Option<String>,

    /// Alipay root certificate path.
    alipay_root_cert_path: Option<String>,

    /// Default asynchronous notification address.
    notify_url: Option<String>,

    /// Default synchronous return address.
    return_url: Option<String>,

    /// App auth token.
    /// 是商家授权给服务商的应用授权凭证。服务商代商家调用支付宝 OpenAPI 时需传入该值。
    /// 仅在服务商代调用场景下使用。在没有重新授权、取消授权或刷新授权的情况下，永久有效。
    app_auth_token: Option<String>,
}

impl AlipayConfig {
    /// Default production gateway.
    pub const DEFAULT_GATEWAY_URL: &str = "https://openapi.alipay.com/gateway.do";

    /// Builds a config with RSA2 defaults.
    pub fn new(
        app_id: impl Into<String>,
        merchant_private_key: impl Into<String>,
        alipay_public_key: impl Into<String>,
    ) -> Self {
        Self {
            app_id: app_id.into(),
            merchant_private_key: merchant_private_key.into(),
            alipay_public_key: alipay_public_key.into(),
            gateway_url: Self::DEFAULT_GATEWAY_URL.to_string(),
            charset: "utf-8".to_string(),
            format: "JSON".to_string(),
            sign_type: "RSA2".to_string(),
            version: "1.0".to_string(),
            merchant_cert_path: None,
            alipay_cert_path: None,
            alipay_root_cert_path: None,
            notify_url: None,
            return_url: None,
            app_auth_token: None,
        }
    }

    /// Sets a custom gateway URL.
    pub fn with_gateway_url(mut self, gateway_url: impl Into<String>) -> Self {
        self.gateway_url = gateway_url.into();
        self
    }

    /// Sets a default notification URL.
    pub fn with_notify_url(mut self, notify_url: impl Into<String>) -> Self {
        self.notify_url = Some(notify_url.into());
        self
    }
}

impl AlipayConfig {
    /// Returns the `app_id` field.
    pub fn app_id(&self) -> &str {
        self.app_id.as_str()
    }

    /// Returns the `merchant_private_key` field.
    pub fn merchant_private_key(&self) -> &str {
        self.merchant_private_key.as_str()
    }

    /// Returns the `alipay_public_key` field.
    pub fn alipay_public_key(&self) -> &str {
        self.alipay_public_key.as_str()
    }

    /// Returns the `gateway_url` field.
    pub fn gateway_url(&self) -> &str {
        self.gateway_url.as_str()
    }

    /// Returns the `charset` field.
    pub fn charset(&self) -> &str {
        self.charset.as_str()
    }

    /// Returns the `format` field.
    pub fn format(&self) -> &str {
        self.format.as_str()
    }

    /// Returns the `sign_type` field.
    pub fn sign_type(&self) -> &str {
        self.sign_type.as_str()
    }

    /// Returns the `version` field.
    pub fn version(&self) -> &str {
        self.version.as_str()
    }

    /// Returns the `notify_url` field.
    pub fn notify_url(&self) -> Option<&str> {
        self.notify_url.as_deref()
    }

    /// Returns the `return_url` field.
    pub fn return_url(&self) -> Option<&str> {
        self.return_url.as_deref()
    }

    /// Returns the `app_auth_token` field.
    pub fn app_auth_token(&self) -> Option<&str> {
        self.app_auth_token.as_deref()
    }

    /// Sets the `app_id` field.
    pub fn set_app_id(&mut self, value: String) {
        self.app_id = value;
    }

    /// Sets the `merchant_private_key` field.
    pub fn set_merchant_private_key(&mut self, value: String) {
        self.merchant_private_key = value;
    }

    /// Sets the `alipay_public_key` field.
    pub fn set_alipay_public_key(&mut self, value: String) {
        self.alipay_public_key = value;
    }

    /// Sets the `gateway_url` field.
    pub fn set_gateway_url(&mut self, value: String) {
        self.gateway_url = value;
    }

    /// Sets the `charset` field.
    pub fn set_charset(&mut self, value: String) {
        self.charset = value;
    }

    /// Sets the `format` field.
    pub fn set_format(&mut self, value: String) {
        self.format = value;
    }

    /// Sets the `sign_type` field.
    pub fn set_sign_type(&mut self, value: String) {
        self.sign_type = value;
    }

    /// Sets the `version` field.
    pub fn set_version(&mut self, value: String) {
        self.version = value;
    }

    /// Sets the `alipay_cert_path` field.
    pub fn set_alipay_cert_path(&mut self, value: String) {
        self.alipay_cert_path = Some(value);
    }

    /// Sets the merchant application public certificate path.
    pub fn set_merchant_cert_path(&mut self, value: String) {
        self.merchant_cert_path = Some(value);
    }

    /// Sets the `alipay_root_cert_path` field.
    pub fn set_alipay_root_cert_path(&mut self, value: String) {
        self.alipay_root_cert_path = Some(value);
    }

    /// Returns the merchant application public certificate path.
    pub fn merchant_cert_path(&self) -> Option<&str> {
        self.merchant_cert_path.as_deref()
    }

    /// Returns the Alipay public certificate path.
    pub fn alipay_cert_path(&self) -> Option<&str> {
        self.alipay_cert_path.as_deref()
    }

    /// Returns the Alipay root certificate path.
    pub fn alipay_root_cert_path(&self) -> Option<&str> {
        self.alipay_root_cert_path.as_deref()
    }

    /// Sets the `notify_url` field.
    pub fn set_notify_url<V>(&mut self, value: V)
    where
        V: Into<Option<String>>,
    {
        self.notify_url = value.into();
    }

    /// Sets the `return_url` field.
    pub fn set_return_url<V>(&mut self, value: V)
    where
        V: Into<Option<String>>,
    {
        self.return_url = value.into();
    }

    /// Sets the `app_auth_token` field.
    pub fn set_app_auth_token<V>(&mut self, value: V)
    where
        V: Into<Option<String>>,
    {
        self.app_auth_token = value.into();
    }
}

impl Debug for AlipayConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlipayConfig")
            .field("app_id", &"***")
            .field("merchant_private_key", &"***")
            .field("alipay_public_key", &"***")
            .field("gateway_url", &self.gateway_url)
            .field("charset", &self.charset)
            .field("format", &self.format)
            .field("sign_type", &self.sign_type)
            .field("version", &self.version)
            .field("merchant_cert_path", &self.merchant_cert_path)
            .field("alipay_cert_path", &self.alipay_cert_path)
            .field("alipay_root_cert_path", &self.alipay_root_cert_path)
            .field("notify_url", &self.notify_url)
            .field("return_url", &self.return_url)
            .field("app_auth_token", &"***")
            .finish()
    }
}

impl Default for AlipayConfig {
    fn default() -> Self {
        Self {
            app_id: std::env::var("ALIPAY_APP_ID").unwrap(),
            merchant_private_key: std::env::var("ALIPAY_APP_PRIVATE_KEY").unwrap(),
            alipay_public_key: std::env::var("ALIPAY_PUBLIC_KEY").unwrap(),
            gateway_url: Self::DEFAULT_GATEWAY_URL.into(),
            charset: "utf-8".into(),
            format: "JSON".into(),
            sign_type: "RSA2".into(),
            version: "1.0".into(),
            merchant_cert_path: Default::default(),
            alipay_cert_path: Default::default(),
            alipay_root_cert_path: Default::default(),
            notify_url: Default::default(),
            return_url: Default::default(),
            app_auth_token: Default::default(),
        }
    }
}
