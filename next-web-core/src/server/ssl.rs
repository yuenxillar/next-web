use serde::{Deserialize, Serialize};

/// SSL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ssl {
    #[serde(default = "default_enabled")]
    enabled: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    client_auth: Option<ClientAuth>,

    #[serde(skip_serializing_if = "Option::is_none")]
    ciphers: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    enabled_protocols: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    key_alias: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    key_password: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    key_store: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    key_store_password: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    key_store_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    key_store_provider: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    trust_store: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    trust_store_password: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    trust_store_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    trust_store_provider: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    certificate: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    certificate_private_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    trust_certificate: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    trust_certificate_private_key: Option<String>,

    #[serde(default = "default_protocol")]
    protocol: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientAuth {
    None,
    Want,
    Need,
}

fn default_enabled() -> bool {
    true
}

fn default_protocol() -> String {
    "TLS".to_string()
}

impl Default for ClientAuth {
    fn default() -> Self {
        ClientAuth::None
    }
}

impl Ssl {
    pub fn new() -> Self {
        Self::default()
    }

    // Getters
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn client_auth(&self) -> Option<ClientAuth> {
        self.client_auth
    }

    pub fn ciphers(&self) -> Option<&[String]> {
        self.ciphers.as_deref()
    }

    pub fn enabled_protocols(&self) -> Option<&[String]> {
        self.enabled_protocols.as_deref()
    }

    pub fn key_alias(&self) -> Option<&str> {
        self.key_alias.as_deref()
    }

    pub fn key_password(&self) -> Option<&str> {
        self.key_password.as_deref()
    }

    pub fn key_store(&self) -> Option<&str> {
        self.key_store.as_deref()
    }

    pub fn key_store_password(&self) -> Option<&str> {
        self.key_store_password.as_deref()
    }

    pub fn key_store_type(&self) -> Option<&str> {
        self.key_store_type.as_deref()
    }

    pub fn key_store_provider(&self) -> Option<&str> {
        self.key_store_provider.as_deref()
    }

    pub fn trust_store(&self) -> Option<&str> {
        self.trust_store.as_deref()
    }

    pub fn trust_store_password(&self) -> Option<&str> {
        self.trust_store_password.as_deref()
    }

    pub fn trust_store_type(&self) -> Option<&str> {
        self.trust_store_type.as_deref()
    }

    pub fn trust_store_provider(&self) -> Option<&str> {
        self.trust_store_provider.as_deref()
    }

    pub fn certificate(&self) -> Option<&str> {
        self.certificate.as_deref()
    }

    pub fn certificate_private_key(&self) -> Option<&str> {
        self.certificate_private_key.as_deref()
    }

    pub fn trust_certificate(&self) -> Option<&str> {
        self.trust_certificate.as_deref()
    }

    pub fn trust_certificate_private_key(&self) -> Option<&str> {
        self.trust_certificate_private_key.as_deref()
    }

    pub fn protocol(&self) -> &str {
        &self.protocol
    }

    // Setters (Builder pattern)
    pub fn set_enabled(&mut self, enabled: bool) -> &mut Self {
        self.enabled = enabled;
        self
    }

    pub fn set_client_auth(&mut self, client_auth: ClientAuth) -> &mut Self {
        self.client_auth = Some(client_auth);
        self
    }

    pub fn set_ciphers(&mut self, ciphers: Vec<String>) -> &mut Self {
        self.ciphers = Some(ciphers);
        self
    }

    pub fn set_enabled_protocols(&mut self, protocols: Vec<String>) -> &mut Self {
        self.enabled_protocols = Some(protocols);
        self
    }

    pub fn set_key_alias(&mut self, alias: impl Into<String>) -> &mut Self {
        self.key_alias = Some(alias.into());
        self
    }

    pub fn set_key_password(&mut self, password: impl Into<String>) -> &mut Self {
        self.key_password = Some(password.into());
        self
    }

    pub fn set_key_store(&mut self, store: impl Into<String>) -> &mut Self {
        self.key_store = Some(store.into());
        self
    }

    pub fn set_key_store_password(&mut self, password: impl Into<String>) -> &mut Self {
        self.key_store_password = Some(password.into());
        self
    }

    pub fn set_key_store_type(&mut self, store_type: impl Into<String>) -> &mut Self {
        self.key_store_type = Some(store_type.into());
        self
    }

    pub fn set_key_store_provider(&mut self, provider: impl Into<String>) -> &mut Self {
        self.key_store_provider = Some(provider.into());
        self
    }

    pub fn set_trust_store(&mut self, store: impl Into<String>) -> &mut Self {
        self.trust_store = Some(store.into());
        self
    }

    pub fn set_trust_store_password(&mut self, password: impl Into<String>) -> &mut Self {
        self.trust_store_password = Some(password.into());
        self
    }

    pub fn set_trust_store_type(&mut self, store_type: impl Into<String>) -> &mut Self {
        self.trust_store_type = Some(store_type.into());
        self
    }

    pub fn set_trust_store_provider(&mut self, provider: impl Into<String>) -> &mut Self {
        self.trust_store_provider = Some(provider.into());
        self
    }

    pub fn set_certificate(&mut self, certificate: impl Into<String>) -> &mut Self {
        self.certificate = Some(certificate.into());
        self
    }

    pub fn set_certificate_private_key(&mut self, key: impl Into<String>) -> &mut Self {
        self.certificate_private_key = Some(key.into());
        self
    }

    pub fn set_trust_certificate(&mut self, certificate: impl Into<String>) -> &mut Self {
        self.trust_certificate = Some(certificate.into());
        self
    }

    pub fn set_trust_certificate_private_key(&mut self, key: impl Into<String>) -> &mut Self {
        self.trust_certificate_private_key = Some(key.into());
        self
    }

    pub fn set_protocol(&mut self, protocol: impl Into<String>) -> &mut Self {
        self.protocol = protocol.into();
        self
    }
}

impl Default for Ssl {
    fn default() -> Self {
        Self {
            enabled: true,
            client_auth: None,
            ciphers: None,
            enabled_protocols: None,
            key_alias: None,
            key_password: None,
            key_store: None,
            key_store_password: None,
            key_store_type: None,
            key_store_provider: None,
            trust_store: None,
            trust_store_password: None,
            trust_store_type: None,
            trust_store_provider: None,
            certificate: None,
            certificate_private_key: None,
            trust_certificate: None,
            trust_certificate_private_key: None,
            protocol: "TLS".to_string(),
        }
    }
}

// Ssl Builder
#[derive(Debug, Clone, Default)]
pub struct SslBuilder {
    ssl: Ssl,
}

impl SslBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.ssl.enabled = enabled;
        self
    }

    pub fn client_auth(mut self, auth: ClientAuth) -> Self {
        self.ssl.client_auth = Some(auth);
        self
    }

    pub fn ciphers(mut self, ciphers: Vec<String>) -> Self {
        self.ssl.ciphers = Some(ciphers);
        self
    }

    pub fn enabled_protocols(mut self, protocols: Vec<String>) -> Self {
        self.ssl.enabled_protocols = Some(protocols);
        self
    }

    pub fn key_alias(mut self, alias: impl Into<String>) -> Self {
        self.ssl.key_alias = Some(alias.into());
        self
    }

    pub fn key_password(mut self, password: impl Into<String>) -> Self {
        self.ssl.key_password = Some(password.into());
        self
    }

    pub fn key_store(mut self, store: impl Into<String>) -> Self {
        self.ssl.key_store = Some(store.into());
        self
    }

    pub fn key_store_password(mut self, password: impl Into<String>) -> Self {
        self.ssl.key_store_password = Some(password.into());
        self
    }

    pub fn key_store_type(mut self, store_type: impl Into<String>) -> Self {
        self.ssl.key_store_type = Some(store_type.into());
        self
    }

    pub fn protocol(mut self, protocol: impl Into<String>) -> Self {
        self.ssl.protocol = protocol.into();
        self
    }

    pub fn build(self) -> Ssl {
        self.ssl
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssl_config() {
        let ssl = SslBuilder::new()
            .enabled(true)
            .client_auth(ClientAuth::Need)
            .key_store("classpath:keystore.p12".to_string())
            .key_store_password("changeit".to_string())
            .key_store_type("PKCS12".to_string())
            .key_alias("tomcat".to_string())
            .protocol("TLSv1.3".to_string())
            .build();

        assert!(ssl.enabled());
        assert_eq!(ssl.client_auth(), Some(ClientAuth::Need));
        assert_eq!(ssl.key_store(), Some("classpath:keystore.p12"));
        assert_eq!(ssl.protocol(), "TLSv1.3");
    }
}
