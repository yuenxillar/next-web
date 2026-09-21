//! A web server that serves an application over HTTPS.
//!
//! The server is available when the crate is built with its `tls-rustls`
//! feature, which enables the TLS support of the server, and is not compiled
//! without it.

use std::{error::Error, fmt, future::Future, io, net::SocketAddr, path::Path, time::Duration};

use axum::Router;
use axum_server::{tls_rustls::RustlsConfig, Handle};
use next_web_core::env::ConfigurableEnvironment;
use tracing::info;

use crate::web::server::{web_server::shutdown_signal, Server};

/// Property that enables TLS for the web server.
///
/// TLS is used when the property is `true`, and it is not used when the property
/// is `false`. When the property is absent, TLS is used when both the certificate
/// and its private key are configured.
pub const SSL_ENABLED_PROPERTY: &str = "next.server.ssl.enabled";

/// Property that names the file that holds the certificate chain of the server,
/// in PEM format.
pub const SSL_CERTIFICATE_PROPERTY: &str = "next.server.ssl.certificate";

/// Property that names the file that holds the private key of the certificate of
/// the server, in PEM format.
pub const SSL_CERTIFICATE_PRIVATE_KEY_PROPERTY: &str = "next.server.ssl.certificate_private_key";

/// The time that the connections of a server are given to complete when it shuts
/// down.
const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// A web server that serves a router over HTTPS.
///
/// The certificate chain and the private key of the server are read from PEM
/// files. [`TlsWebServer::from_environment`] reads the names of those files from
/// the [`SSL_CERTIFICATE_PROPERTY`] and
/// [`SSL_CERTIFICATE_PRIVATE_KEY_PROPERTY`] properties, while the other
/// constructors take the certificate and the key directly.
pub struct TlsWebServer {
    socket_addr: SocketAddr,
    router: Option<Router>,
    tls_config: RustlsConfig,
}

impl TlsWebServer {
    /// Returns whether TLS is enabled for the given environment.
    ///
    /// The [`SSL_ENABLED_PROPERTY`] property decides. When it is absent, TLS is
    /// enabled when both the certificate and its private key are configured.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to read the configuration from.
    pub fn is_enabled(environment: &dyn ConfigurableEnvironment) -> bool {
        match environment.get_property(SSL_ENABLED_PROPERTY) {
            Some(value) => parse_bool(&value).unwrap_or(false),
            None => {
                environment.get_property(SSL_CERTIFICATE_PROPERTY).is_some()
                    && environment
                        .get_property(SSL_CERTIFICATE_PRIVATE_KEY_PROPERTY)
                        .is_some()
            }
        }
    }

    /// Creates a server that serves the given router over HTTPS, using a
    /// certificate chain and a private key that are held in memory.
    ///
    /// # Arguments
    ///
    /// * `socket_addr` - The address the server binds to.
    /// * `router` - The router to serve.
    /// * `certificate` - The certificate chain, in PEM format.
    /// * `private_key` - The private key of the certificate, in PEM format.
    ///
    /// # Errors
    ///
    /// Returns [`TlsWebServerError::Load`] when the certificate or the private key
    /// is not valid.
    pub async fn from_pem(
        socket_addr: impl Into<SocketAddr>,
        router: Router,
        certificate: impl Into<Vec<u8>>,
        private_key: impl Into<Vec<u8>>,
    ) -> Result<Self, TlsWebServerError> {
        let tls_config = RustlsConfig::from_pem(certificate.into(), private_key.into())
            .await
            .map_err(TlsWebServerError::Load)?;

        Ok(Self::new(socket_addr.into(), router, tls_config))
    }

    /// Creates a server that serves the given router over HTTPS, using the
    /// certificate chain and the private key that are stored in the given files,
    /// both in PEM format.
    ///
    /// # Arguments
    ///
    /// * `socket_addr` - The address the server binds to.
    /// * `router` - The router to serve.
    /// * `certificate` - The file that holds the certificate chain.
    /// * `private_key` - The file that holds the private key of the certificate.
    ///
    /// # Errors
    ///
    /// Returns [`TlsWebServerError::Load`] when a file cannot be read or does not
    /// hold a valid certificate or private key.
    pub async fn from_pem_file(
        socket_addr: impl Into<SocketAddr>,
        router: Router,
        certificate: impl AsRef<Path>,
        private_key: impl AsRef<Path>,
    ) -> Result<Self, TlsWebServerError> {
        let tls_config = RustlsConfig::from_pem_file(certificate, private_key)
            .await
            .map_err(TlsWebServerError::Load)?;

        Ok(Self::new(socket_addr.into(), router, tls_config))
    }

    /// Creates a server that serves the given router over HTTPS, using the TLS
    /// configuration of the given environment.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to read the configuration from.
    /// * `socket_addr` - The address the server binds to.
    /// * `router` - The router to serve.
    ///
    /// # Errors
    ///
    /// Returns [`TlsWebServerError::MissingKeyMaterial`] when the certificate or
    /// the private key is not configured, and [`TlsWebServerError::Load`] when it
    /// cannot be read or is not valid.
    pub async fn from_environment(
        environment: &dyn ConfigurableEnvironment,
        socket_addr: impl Into<SocketAddr>,
        router: Router,
    ) -> Result<Self, TlsWebServerError> {
        let certificate = environment
            .get_property(SSL_CERTIFICATE_PROPERTY)
            .ok_or(TlsWebServerError::MissingKeyMaterial)?;
        let private_key = environment
            .get_property(SSL_CERTIFICATE_PRIVATE_KEY_PROPERTY)
            .ok_or(TlsWebServerError::MissingKeyMaterial)?;

        Self::from_pem_file(socket_addr, router, certificate, private_key).await
    }

    /// Creates a server for a configuration that has already been loaded.
    fn new(socket_addr: SocketAddr, router: Router, tls_config: RustlsConfig) -> Self {
        Self {
            socket_addr,
            router: Some(router),
            tls_config,
        }
    }

    /// Serves the router over HTTPS until the server shuts down.
    async fn serve(&mut self) -> Result<(), Box<dyn Error>> {
        let app = self.router.take().unwrap_or_else(Router::new);

        // The connections that are still open are given a moment to complete
        // once the application receives a shutdown signal.
        let handle = Handle::new();
        spawn_shutdown_listener(handle.clone());

        let mut server =
            axum_server::bind_rustls(self.socket_addr, self.tls_config.clone()).handle(handle);
        // Advertise the support for HTTP/2 websockets to the client, which
        // `axum::serve` does by default.
        server.http_builder().http2().enable_connect_protocol();

        server
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await?;

        Ok(())
    }
}

impl Server for TlsWebServer {
    /// Serves the router over HTTPS until the server shuts down.
    fn run<'a>(&'a mut self) -> impl Future<Output = Result<(), Box<dyn Error>>> + 'a {
        async move { self.serve().await }
    }
}

impl fmt::Debug for TlsWebServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TlsWebServer")
            .field("socket_addr", &self.socket_addr)
            .finish_non_exhaustive()
    }
}

/// Error reported when a TLS server cannot be created or run.
#[derive(Debug)]
pub enum TlsWebServerError {
    /// TLS is enabled, but the certificate or the private key of the server is
    /// not configured.
    MissingKeyMaterial,
    /// The certificate or the private key of the server could not be read or is
    /// not valid.
    Load(io::Error),
}

impl fmt::Display for TlsWebServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKeyMaterial => write!(
                f,
                "TLS is enabled, but {SSL_CERTIFICATE_PROPERTY} or \
                 {SSL_CERTIFICATE_PRIVATE_KEY_PROPERTY} is not configured"
            ),
            Self::Load(error) => write!(
                f,
                "The certificate or the private key of the server could not be loaded: {error}"
            ),
        }
    }
}

impl Error for TlsWebServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Load(error) => Some(error),
            Self::MissingKeyMaterial => None,
        }
    }
}

/// Spawns the task that shuts the server down gracefully once the application
/// receives a shutdown signal.
///
/// # Arguments
///
/// * `handle` - The handle that shuts the connections of the server down.
fn spawn_shutdown_listener(handle: Handle<SocketAddr>) {
    tokio::spawn(async move {
        let reason = shutdown_signal().await;
        info!("Graceful shutdown of application completed, reason: {reason}");
        handle.graceful_shutdown(Some(GRACEFUL_SHUTDOWN_TIMEOUT));
    });
}

/// Parses a boolean property value, accepting `true` and `false` in any case.
///
/// # Arguments
///
/// * `value` - The value to parse.
fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::env::{ConfigurableEnvironment, StandardEnvironment};
    use next_web_core::util::indexmap::IndexMap;

    use crate::env::MapPropertySource;

    use super::*;

    /// Returns an environment holding the given properties.
    fn environment(properties: &[(&str, &str)]) -> StandardEnvironment {
        let mut environment = StandardEnvironment::new();
        let properties: IndexMap<String, String> = properties
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        environment
            .property_sources()
            .add_first(Box::new(MapPropertySource::new(
                "test".to_owned(),
                properties,
            )));
        environment
    }

    /// Returns an address that a test server can bind to.
    fn socket_addr() -> SocketAddr {
        "127.0.0.1:0".parse::<SocketAddr>().unwrap()
    }

    #[test]
    fn tls_is_disabled_without_a_configuration() {
        assert!(!TlsWebServer::is_enabled(&environment(&[])));
    }

    #[test]
    fn tls_is_enabled_by_its_property() {
        assert!(TlsWebServer::is_enabled(&environment(&[(
            SSL_ENABLED_PROPERTY,
            "TRUE"
        )])));
        assert!(!TlsWebServer::is_enabled(&environment(&[(
            SSL_ENABLED_PROPERTY,
            "false"
        )])));
    }

    #[test]
    fn tls_is_enabled_by_a_certificate_and_its_private_key() {
        assert!(TlsWebServer::is_enabled(&environment(&[
            (SSL_CERTIFICATE_PROPERTY, "cert.pem"),
            (SSL_CERTIFICATE_PRIVATE_KEY_PROPERTY, "key.pem"),
        ])));
        assert!(!TlsWebServer::is_enabled(&environment(&[(
            SSL_CERTIFICATE_PROPERTY,
            "cert.pem"
        )])));
        assert!(!TlsWebServer::is_enabled(&environment(&[(
            SSL_CERTIFICATE_PRIVATE_KEY_PROPERTY,
            "key.pem"
        )])));
    }

    #[test]
    fn a_certificate_does_not_enable_tls_when_it_is_disabled() {
        assert!(!TlsWebServer::is_enabled(&environment(&[
            (SSL_ENABLED_PROPERTY, "false"),
            (SSL_CERTIFICATE_PROPERTY, "cert.pem"),
            (SSL_CERTIFICATE_PRIVATE_KEY_PROPERTY, "key.pem"),
        ])));
    }

    #[tokio::test]
    async fn requires_the_certificate_and_the_private_key_of_the_environment() {
        let error = TlsWebServer::from_environment(
            &environment(&[(SSL_CERTIFICATE_PROPERTY, "cert.pem")]),
            socket_addr(),
            Router::new(),
        )
        .await
        .unwrap_err();

        assert!(matches!(error, TlsWebServerError::MissingKeyMaterial));
        assert!(error
            .to_string()
            .contains(SSL_CERTIFICATE_PRIVATE_KEY_PROPERTY));
    }

    #[test]
    fn reports_the_reason_of_a_failure() {
        let error = TlsWebServerError::MissingKeyMaterial;
        assert!(error.to_string().contains(SSL_CERTIFICATE_PROPERTY));
        assert!(error.source().is_none());

        let error = TlsWebServerError::Load(io::Error::other("unreadable"));
        assert!(error.to_string().contains("unreadable"));
        assert!(error.source().is_some());
    }

    #[tokio::test]
    async fn reports_a_certificate_that_cannot_be_loaded() {
        let error = TlsWebServer::from_pem(
            socket_addr(),
            Router::new(),
            "not a certificate".as_bytes().to_vec(),
            "not a private key".as_bytes().to_vec(),
        )
        .await
        .unwrap_err();

        assert!(matches!(error, TlsWebServerError::Load(_)));
    }
}
