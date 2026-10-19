//! A web server that serves an application over HTTPS.
//!
//! The server is available when the crate is built with its `tls-rustls`
//! feature, which enables the TLS support of the server, and is not compiled
//! without it.

use std::{
    error::Error,
    fmt, future::Future,
    io,
    net::SocketAddr,
    path::Path,
    sync::Arc,
};

use axum::{
    Router,
    extract::{ConnectInfo, Request, connect_info::Connected},
    middleware::{self, Next},
    response::Response,
    serve::{IncomingStream, Listener},
};
use next_web_core::{env::ConfigurableEnvironment, http::PeerCertificate};
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use tokio::net::{TcpListener, TcpStream};

use crate::web::server::{Server, web_server::graceful_shutdown};

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

/// A web server that serves a router over HTTPS.
///
/// The certificate chain and the private key of the server are read from PEM
/// files. [`TlsWebServer::from_environment`] reads the names of those files from
/// the [`SSL_CERTIFICATE_PROPERTY`] and
/// [`SSL_CERTIFICATE_PRIVATE_KEY_PROPERTY`] properties, while the other
/// constructors take the certificate and the key directly.
///
/// The connection of a client is settled before the router sees a request, and
/// the certificate the client presented is part of that connection. The server
/// inserts it into every request as [`PeerCertificate`], and the certificate of
/// a client is what the X.509 support authenticates.
pub struct TlsWebServer {
    socket_addr: SocketAddr,
    router: Option<Router>,
    server_config: Arc<rustls::ServerConfig>,
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
        let server_config = server_config_from_pem(certificate.into(), private_key.into())?;

        Ok(Self::new(socket_addr.into(), router, server_config))
    }

    /// Creates a server that serves the given router over the given TLS
    /// configuration.
    ///
    /// The configuration is the place an application asks for a client
    /// certificate, since a verifier of client certificates is part of it.
    ///
    /// # Arguments
    ///
    /// * `socket_addr` - The address the server binds to.
    /// * `router` - The router to serve.
    /// * `server_config` - The TLS configuration of the server.
    pub fn from_config(
        socket_addr: impl Into<SocketAddr>,
        router: Router,
        server_config: Arc<rustls::ServerConfig>,
    ) -> Self {
        Self::new(socket_addr.into(), router, server_config)
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
        let certificate = tokio::fs::read(certificate).await.map_err(TlsWebServerError::Load)?;
        let private_key = tokio::fs::read(private_key).await.map_err(TlsWebServerError::Load)?;
        let server_config = server_config_from_pem(certificate, private_key)?;

        Ok(Self::new(socket_addr.into(), router, server_config))
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
    fn new(socket_addr: SocketAddr, router: Router, server_config: Arc<rustls::ServerConfig>) -> Self {
        Self {
            socket_addr,
            router: Some(router),
            server_config,
        }
    }

    /// Serves the router over HTTPS until the server shuts down.
    async fn serve(&mut self) -> Result<(), Box<dyn Error>> {
        let app = self.router.take().unwrap_or_else(Router::new);

        let listener = TcpListener::bind(self.socket_addr).await?;
        let acceptor = TlsAcceptor::from(self.server_config.clone());

        // The certificate of a connection is handed to the request it carries,
        // after the handshake that settled it and before the router sees it: no
        // layer behind the server can reach a certificate, and an application
        // that authenticates one would never see it.
        let app = app.layer(middleware::from_fn(hand_over_certificate));

        axum::serve(
            TlsListener { listener, acceptor },
            app.into_make_service_with_connect_info::<TlsPeer>(),
        )
        .with_graceful_shutdown(graceful_shutdown())
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

/// What the TLS connection of a request carried.
///
/// The address is the one of the client, and the certificate is the one the
/// handshake settled on, encoded the way it arrived.
#[derive(Clone, Debug)]
pub struct TlsPeer {
    addr: SocketAddr,
    certificate: Option<Vec<u8>>,
}

impl Connected<IncomingStream<'_, TlsListener>> for TlsPeer {
    fn connect_info(stream: IncomingStream<'_, TlsListener>) -> Self {
        stream.remote_addr().clone()
    }
}

/// A listener that settles the TLS connection of every client before the
/// router sees the request it carries.
struct TlsListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
}

impl Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = TlsPeer;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (stream, addr) = match self.listener.accept().await {
                Ok(accepted) => accepted,
                Err(error) => {
                    tracing::warn!("the connection of a client failed: {error}");
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    continue;
                }
            };

            match self.acceptor.accept(stream).await {
                Ok(stream) => {
                    let certificate = stream
                        .get_ref()
                        .1
                        .peer_certificates()
                        .and_then(|certificates| certificates.first())
                        .map(|certificate| certificate.as_ref().to_vec());

                    return (stream, TlsPeer { addr, certificate });
                }
                Err(error) => {
                    tracing::debug!("the TLS connection of {addr} failed: {error}");
                }
            }
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        let addr = self.listener.local_addr()?;

        Ok(TlsPeer {
            addr,
            certificate: None,
        })
    }
}

/// Hands the certificate of the connection to the request it carries.
///
/// The request gets the address of the client, and the certificate when the
/// handshake settled on one. Nothing in the request itself proves a certificate,
/// so only this server — which made the connection and verified the certificate
/// against the configuration it was given — inserts one.
async fn hand_over_certificate(mut request: Request, next: Next) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<TlsPeer>>()
        .map(|info| info.0.clone());

    if let Some(peer) = peer {
        request.extensions_mut().insert(peer.addr);
        if let Some(certificate) = peer.certificate {
            request
                .extensions_mut()
                .insert(PeerCertificate::new(certificate));
        }
    }

    next.run(request).await
}

/// Returns the TLS configuration of a server from a certificate chain and the
/// private key of its certificate, both in PEM format.
///
/// # Errors
///
/// Returns [`TlsWebServerError::Load`] when either document is not a PEM
/// document, or when they do not belong together.
fn server_config_from_pem(
    certificate: Vec<u8>,
    private_key: Vec<u8>,
) -> Result<Arc<rustls::ServerConfig>, TlsWebServerError> {
    let certificates = rustls_pemfile::certs(&mut certificate.as_slice())
        .collect::<Result<Vec<_>, _>>()
        .map_err(TlsWebServerError::Load)?;
    let private_key = rustls_pemfile::private_key(&mut private_key.as_slice())
        .map_err(TlsWebServerError::Load)?
        .ok_or_else(|| {
            TlsWebServerError::Load(io::Error::new(
                io::ErrorKind::InvalidData,
                "the private key of the server is missing",
            ))
        })?;
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certificates, private_key)
        .map_err(|error| TlsWebServerError::Load(io::Error::new(io::ErrorKind::InvalidData, error)))?;

    Ok(Arc::new(config))
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
        assert!(
            error
                .to_string()
                .contains(SSL_CERTIFICATE_PRIVATE_KEY_PROPERTY)
        );
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
