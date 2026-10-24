/// WeChat Pay V2 configuration.
#[derive(Debug, Clone)]
pub struct WechatPayConfig {
    /// WeChat app id.
    pub app_id: String,

    /// Merchant id.
    pub mch_id: String,

    /// API key configured in the merchant platform.
    pub api_key: String,

    /// Default notify URL.
    pub notify_url: Option<String>,

    /// Gateway base URL.
    pub base_url: String,

    /// PKCS#12 certificate path for refund and reverse APIs.
    pub merchant_cert_p12_path: Option<String>,

    /// PKCS#12 certificate password. Usually the merchant id.
    pub merchant_cert_p12_password: Option<String>,

    /// V3 API: merchant certificate serial number for Authorization header.
    pub merchant_serial_no: Option<String>,
    
    /// V3 API: merchant private key (PEM format) for signing.
    pub merchant_private_key: Option<String>,
}

impl WechatPayConfig {
    /// Production V2 gateway base URL.
    pub const DEFAULT_BASE_URL: &str = "https://api.mch.weixin.qq.com";

    /// Creates a config with MD5 signing by default.
    pub fn new(
        app_id: impl Into<String>,
        mch_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            app_id: app_id.into(),
            mch_id: mch_id.into(),
            api_key: api_key.into(),
            notify_url: None,
            base_url: Self::DEFAULT_BASE_URL.to_string(),
            merchant_cert_p12_path: None,
            merchant_cert_p12_password: None,
            merchant_serial_no: None,
            merchant_private_key: None,
        }
    }


    /// Sets the default notify URL.
    pub fn with_notify_url(mut self, notify_url: impl Into<String>) -> Self {
        self.notify_url = Some(notify_url.into());
        self
    }

    /// Sets a custom gateway base URL.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Configures the merchant PKCS#12 certificate used by certificate APIs.
    pub fn with_merchant_cert(
        mut self,
        path: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.merchant_cert_p12_path = Some(path.into());
        self.merchant_cert_p12_password = Some(password.into());
        self
    }

    /// Sets the V3 API merchant certificate serial number.
    pub fn with_merchant_serial_no(mut self, serial_no: impl Into<String>) -> Self {
        self.merchant_serial_no = Some(serial_no.into());
        self
    }

    /// Sets the V3 API merchant private key (PEM format).
    pub fn with_merchant_private_key(mut self, private_key: impl Into<String>) -> Self {
        self.merchant_private_key = Some(private_key.into());
        self
    }
}

impl WechatPayConfig {
    pub fn base_url(&self) -> &str {
        self.base_url.as_str()
    }
}
