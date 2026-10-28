//! The TLS configuration of the web server.
//!
//! The properties are part of [`ServerProperties`](super::ServerProperties) and
//! are bound from the `next.server.ssl` prefix of the environment of the
//! application:
//!
//! ```yaml
//! next:
//!   server:
//!     ssl:
//!       enabled: true
//!       certificate: certs/server.crt
//!       certificate_private_key: certs/server.key
//! ```
//!
//! The certificate and its private key are the names of files that are read
//! when the server starts, both of them in PEM format.

use serde::{Deserialize, Serialize};

/// The TLS configuration of the web server.
///
/// The configuration is enabled when the environment says so, and when it does
/// not say anything the server enables TLS as soon as both the certificate and
/// its private key are configured, see [`SslProperties::is_enabled`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SslProperties {
    /// Whether the server serves the application over TLS.
    ///
    /// It defaults to `None`, which leaves the decision to the files the
    /// environment configures. `true` serves the application over HTTPS, which
    /// requires the certificate and its private key to be configured, and
    /// `false` serves it over HTTP whatever the files say.
    enabled: Option<bool>,

    /// The name of the file that holds the certificate chain of the server, in
    /// PEM format.
    certificate: Option<String>,

    /// The name of the file that holds the private key of the certificate of
    /// the server, in PEM format.
    certificate_private_key: Option<String>,
}

impl SslProperties {
    /// Creates a TLS configuration from the files that hold the certificate
    /// chain and its private key.
    ///
    /// # Arguments
    ///
    /// * `certificate` - The name of the file that holds the certificate chain.
    /// * `certificate_private_key` - The name of the file that holds the
    ///   private key of the certificate.
    pub fn new(certificate: impl Into<String>, certificate_private_key: impl Into<String>) -> Self {
        Self {
            enabled: None,
            certificate: Some(certificate.into()),
            certificate_private_key: Some(certificate_private_key.into()),
        }
    }

    /// Returns whether the server serves the application over TLS.
    ///
    /// The configured value decides. When the environment configures none, TLS
    /// is used as soon as both the certificate and its private key are
    /// configured, which is the same rule the TLS server applies to the
    /// properties it reads.
    pub fn is_enabled(&self) -> bool {
        match self.enabled {
            Some(enabled) => enabled,
            None => self.certificate.is_some() && self.certificate_private_key.is_some(),
        }
    }

    /// Returns the value the environment configured for the TLS of the server,
    /// when it configured one.
    ///
    /// The value is `None` when the environment leaves the decision to the
    /// configured files.
    pub fn enabled(&self) -> Option<bool> {
        self.enabled
    }

    /// Returns the name of the file that holds the certificate chain of the
    /// server, when the environment configures one.
    pub fn certificate(&self) -> Option<&str> {
        self.certificate.as_deref()
    }

    /// Returns the name of the file that holds the private key of the
    /// certificate of the server, when the environment configures one.
    pub fn certificate_private_key(&self) -> Option<&str> {
        self.certificate_private_key.as_deref()
    }

    /// Sets whether the server serves the application over TLS.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = Some(enabled);
    }

    /// Sets the name of the file that holds the certificate chain of the
    /// server.
    pub fn set_certificate(&mut self, certificate: impl Into<String>) {
        self.certificate = Some(certificate.into());
    }

    /// Sets the name of the file that holds the private key of the certificate
    /// of the server.
    pub fn set_certificate_private_key(&mut self, certificate_private_key: impl Into<String>) {
        self.certificate_private_key = Some(certificate_private_key.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_is_disabled_when_nothing_is_configured() {
        let properties = SslProperties::default();

        assert_eq!(properties.enabled(), None);
        assert_eq!(properties.certificate(), None);
        assert_eq!(properties.certificate_private_key(), None);
        assert!(!properties.is_enabled());
    }

    #[test]
    fn tls_is_enabled_by_the_files_when_the_environment_does_not_decide() {
        let properties = SslProperties::new("certs/server.crt", "certs/server.key");

        assert_eq!(properties.enabled(), None);
        assert!(properties.is_enabled());
    }

    #[test]
    fn the_configured_value_of_the_environment_wins_over_the_files() {
        let mut properties = SslProperties::new("certs/server.crt", "certs/server.key");
        properties.set_enabled(false);

        assert_eq!(properties.enabled(), Some(false));
        assert!(!properties.is_enabled());
    }

    #[test]
    fn the_settings_are_read_from_a_document() {
        let properties: SslProperties = serde_yaml::from_str(
            r#"
            enabled: true
            certificate: certs/server.crt
            certificate_private_key: certs/server.key
            "#,
        )
        .expect("the document describes the TLS configuration");

        assert_eq!(properties.enabled(), Some(true));
        assert_eq!(properties.certificate(), Some("certs/server.crt"));
        assert_eq!(
            properties.certificate_private_key(),
            Some("certs/server.key")
        );
    }
}
