//! The properties of the web server of an application.
//!
//! The properties are bound from the `next.server` prefix of the environment of
//! the application, and they are registered as a singleton of the application
//! context, so a component can depend on them like on any other singleton:
//!
//! ```yaml
//! next:
//!   server:
//!     port: 8080
//!     address: 0.0.0.0
//!     context_path: /app
//!     http:
//!       request:
//!         max_request_size: 10485760
//!         timeout: 5
//!     ssl:
//!       enabled: true
//!       certificate: certs/server.crt
//!       certificate_private_key: certs/server.key
//! ```
//!
//! A field is read from the property of the same name, written in snake case,
//! in kebab case, or as the name of an environment variable, so the port above
//! is also read from `next.server.port`, `next.server-port` and
//! `NEXT_SERVER_PORT`. The values of the placeholders of a property are
//! resolved against the environment as well, so `address: ${SERVER_ADDRESS}`
//! works. See [`Binder`](next_web_core::env::Binder) for the rules of the
//! binding.
//!
//! The properties also implement [`Deserialize`], so a document that is written
//! by hand, or that is read from a configuration server, can be read into the
//! struct, and [`Serialize`], so the effective configuration of an application
//! can be written back out.

use std::{net::SocketAddr, sync::OnceLock};

use next_web_core::{
    constants::application_constants::APPLICATION_DEFAULT_PORT,
    env::{BindError, Binder, ConfigurableEnvironment},
};
use next_web_macros::configuration_properties;
use serde::{Deserialize, Serialize};

use super::{HttpProperties, SslProperties};

/// The prefix the properties of the web server are bound from.
///
/// The attribute of [`ServerProperties`] declares the same prefix, and the
/// constant is what the properties are bound from when they are read without
/// the attribute, see [`ServerProperties::from_environment`].
pub const SERVER_PROPERTIES_PREFIX: &str = "next.server";

/// The port the server binds when the environment configures no port.
pub const DEFAULT_PORT: u16 = APPLICATION_DEFAULT_PORT;

/// The address the server binds when the environment configures
pub const DEFAULT_ADDRESS: &str = "0.0.0.0";

/// The properties of the web server of the application.
///
/// The properties are bound from the `next.server` prefix of the environment
/// while the application starts, and the instance is registered as a singleton
/// named `serverProperties`. Every value that the environment does not
/// configure falls back to the default of its field, so an application that
/// declares none of them binds the defaults.
///
/// # Examples
///
/// Reading the properties of an environment, which is what the application does
/// while it starts:
///
/// ```ignore
/// let properties = ServerProperties::from_environment(environment)?;
///
/// let socket_addr = properties.socket_addr()?;
/// ```
///
/// Reading the properties of a document:
///
/// ```
/// use next_web::web::server::autoconfigure::ServerProperties;
///
/// let properties: ServerProperties = serde_yaml::from_str(
///     r#"
///     port: 8080
///     context_path: /app
///     "#,
/// )
/// .unwrap();
///
/// assert_eq!(properties.port(), 8080);
/// assert_eq!(properties.normalized_context_path().as_deref(), Some("/app"));
/// ```
#[configuration_properties(prefix = "next.server")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerProperties {
    /// The port the server listens on.
    ///
    /// Defaults to [`DEFAULT_PORT`] when the environment configures no port.
    port: u16,

    /// The interface the server binds.
    ///
    /// The address is the [`SocketAddr`] of the server without its port, so it
    /// is an IPv4 or an IPv6 literal, and `0.0.0.0` binds every interface of
    /// the machine. When the environment configures no address, the server
    /// binds [`DEFAULT_ADDRESS`]
    address: Option<String>,

    /// The path every route of the application is nested under.
    ///
    /// The path is normalized before it is used, so `app`, `/app` and `/app/`
    /// all nest the routes under `/app`, and an empty path, or `/`, nests them
    /// under the root, see [`ServerProperties::normalized_context_path`].
    ///
    /// The path is read from `context_path` and from `context-path` alike, so a
    /// document that is written in either spelling describes the same server.
    context_path: Option<String>,

    /// The HTTP configuration of the server.
    http: HttpProperties,

    /// The TLS configuration of the server.
    ssl: SslProperties,
}

impl ServerProperties {
    /// Creates properties with the given values and the defaults of the HTTP
    /// and TLS configuration.
    ///
    /// # Arguments
    ///
    /// * `port` - The port the server listens on.
    /// * `address` - The interface the server binds.
    /// * `context_path` - The path every route is nested under.
    pub fn new(port: u16, address: impl Into<String>, context_path: impl Into<String>) -> Self {
        Self {
            port,
            address: Some(address.into()),
            context_path: Some(context_path.into()),
            ..Self::default()
        }
    }

    /// Reads the properties from the environment, using the default of a field
    /// the environment does not configure.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment the properties are bound from.
    ///
    /// # Errors
    ///
    /// Returns [`ServerPropertiesError`] when a property of the environment
    /// cannot be read as the type of its field, for example when `port` holds
    /// text that is not a number.
    pub fn from_environment(
        environment: &dyn ConfigurableEnvironment,
    ) -> Result<Self, ServerPropertiesError> {
        Binder::new(environment, SERVER_PROPERTIES_PREFIX)
            .bind::<Self>()
            .map_err(ServerPropertiesError::from)
    }

    /// Installs these properties as the properties of the process.
    ///
    /// The application installs the properties it bound while it started, which
    /// is what lets a server that runs without an application context read the
    /// configuration of the application, see
    /// [`ServerProperties::global`].
    ///
    /// # Errors
    ///
    /// Returns [`ServerPropertiesErrorKind::AlreadyInstalled`] when properties
    /// were installed by another part of the process already.
    pub fn install_global(self) -> Result<(), ServerPropertiesError> {
        GLOBAL_SERVER_PROPERTIES.set(self).map_err(|_| {
            ServerPropertiesError::new(
                ServerPropertiesErrorKind::AlreadyInstalled,
                SERVER_PROPERTIES_PREFIX,
                "the properties of the server are installed already",
            )
        })
    }

    /// Returns the properties that were installed for the process, when they
    /// were.
    pub fn global() -> Option<&'static ServerProperties> {
        GLOBAL_SERVER_PROPERTIES.get()
    }

    /// Returns the port the server listens on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Returns the interface the environment configured, when it configured
    /// one.
    ///
    /// The server binds [`ServerProperties::bind_address`], which falls back to
    /// the default of the environment.
    pub fn address(&self) -> Option<&str> {
        self.address.as_deref()
    }

    /// Returns the interface the server binds.
    ///
    /// The configured address wins. When the environment configures none.
    pub fn bind_address(&self) -> &str {
        match self.address.as_deref() {
            Some(address) => address,
            None => DEFAULT_ADDRESS,
        }
    }

    /// Returns the address the server binds, which is the interface of
    /// [`ServerProperties::bind_address`] and the port of the properties.
    ///
    /// # Errors
    ///
    /// Returns [`ServerPropertiesError`] when the address is not an IP literal,
    /// or when the port cannot be bound with it.
    pub fn socket_addr(&self) -> Result<SocketAddr, ServerPropertiesError> {
        let address = format!("{}:{}", self.bind_address(), self.port);

        address.parse::<SocketAddr>().map_err(|error| {
            ServerPropertiesError::new(
                ServerPropertiesErrorKind::InvalidAddress,
                address.as_str(),
                error.to_string(),
            )
        })
    }

    /// Returns the path the environment configured, when it configured one.
    pub fn context_path(&self) -> Option<&str> {
        self.context_path.as_deref()
    }

    /// Returns the path every route of the application is nested under.
    ///
    /// Returns `None` when the environment configures no path, or when the path
    /// it configures is the root.
    pub fn normalized_context_path(&self) -> Option<String> {
        self.context_path
            .as_deref()
            .and_then(Self::normalize_context_path)
    }

    /// Normalizes a context path, which removes the whitespace of the path, and
    /// gives it a leading slash and no trailing slash.
    ///
    /// Returns `None` when nothing is left of the path, or when the path is the
    /// root, both of which nest the routes of the application under the root,
    /// which is the same as nesting them under nothing.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to normalize.
    pub fn normalize_context_path(path: &str) -> Option<String> {
        let cleaned: String = path.chars().filter(|c| !c.is_ascii_whitespace()).collect();

        if cleaned.is_empty() {
            return None;
        }

        let mut path = if cleaned.starts_with('/') {
            cleaned
        } else {
            format!("/{cleaned}")
        };

        while path.len() > 1 && path.ends_with('/') {
            path.pop();
        }

        if path == "/" {
            return None;
        }

        Some(path)
    }

    /// Returns the HTTP configuration of the server.
    pub fn http(&self) -> &HttpProperties {
        &self.http
    }

    /// Returns the TLS configuration of the server.
    pub fn ssl(&self) -> &SslProperties {
        &self.ssl
    }

    /// Sets the port the server listens on.
    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    /// Sets the interface the server binds.
    pub fn set_address(&mut self, address: impl Into<String>) {
        self.address = Some(address.into());
    }

    /// Sets the path every route of the application is nested under.
    pub fn set_context_path(&mut self, context_path: impl Into<String>) {
        self.context_path = Some(context_path.into());
    }

    /// Replaces the HTTP configuration of the server.
    pub fn set_http(&mut self, http: HttpProperties) {
        self.http = http;
    }

    /// Replaces the TLS configuration of the server.
    pub fn set_ssl(&mut self, ssl: SslProperties) {
        self.ssl = ssl;
    }
}

impl Default for ServerProperties {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            address: None,
            context_path: None,
            http: HttpProperties::default(),
            ssl: SslProperties::default(),
        }
    }
}

impl TryFrom<&dyn ConfigurableEnvironment> for ServerProperties {
    type Error = ServerPropertiesError;

    fn try_from(environment: &dyn ConfigurableEnvironment) -> Result<Self, Self::Error> {
        Self::from_environment(environment)
    }
}

impl From<BindError> for ServerPropertiesError {
    fn from(error: BindError) -> Self {
        let key = error
            .path()
            .map(|path| format!("{SERVER_PROPERTIES_PREFIX}.{path}"));

        Self {
            kind: ServerPropertiesErrorKind::InvalidValue,
            key,
            message: error.message().to_owned(),
        }
    }
}

/// The properties the application was configured with.
///
/// The application installs the properties it bound by calling
/// [`ServerProperties::install_global`] while it starts, and the server of the
/// application reads them. They are not installed by default, so a process that
/// never installs them reads `None`.
pub static GLOBAL_SERVER_PROPERTIES: OnceLock<ServerProperties> = OnceLock::new();

/// The kind of a [`ServerPropertiesError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ServerPropertiesErrorKind {
    /// A property of the environment cannot be read as the type of its field.
    InvalidValue,

    /// The address the server would bind is not one a socket address can hold.
    InvalidAddress,

    /// The properties of the server are installed for the process already.
    AlreadyInstalled,
}

impl ServerPropertiesErrorKind {
    /// Returns a description of this kind.
    pub fn description(&self) -> &'static str {
        match self {
            ServerPropertiesErrorKind::InvalidValue => {
                "a property of the server is not a value its field can hold"
            }
            ServerPropertiesErrorKind::InvalidAddress => {
                "the address of the server is not an IP address and a port"
            }
            ServerPropertiesErrorKind::AlreadyInstalled => {
                "the properties of the server are installed already"
            }
        }
    }
}

impl std::fmt::Display for ServerPropertiesErrorKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.description())
    }
}

/// The error returned when the properties of the web server cannot be read.
///
/// The error carries the [`kind`](ServerPropertiesError::kind) of the failure,
/// which a caller compares to decide how to react to it, the
/// [`key`](ServerPropertiesError::key) of the property the failure occurred at
/// when there is one, and the [`message`](ServerPropertiesError::message) that
/// describes the failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerPropertiesError {
    kind: ServerPropertiesErrorKind,
    key: Option<String>,
    message: String,
}

impl ServerPropertiesError {
    /// Creates an error of the given kind.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of the failure.
    /// * `key` - The property the failure occurred at.
    /// * `message` - The message that describes the failure.
    pub fn new(
        kind: ServerPropertiesErrorKind,
        key: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            key: Some(key.into()),
            message: message.into(),
        }
    }

    /// Returns the kind of this error.
    pub fn kind(&self) -> ServerPropertiesErrorKind {
        self.kind
    }

    /// Returns the property this error occurred at, when it is known.
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    /// Returns the message of this error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for ServerPropertiesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.key {
            Some(key) => write!(formatter, "{}: {}: {}", self.kind, key, self.message),
            None => write!(formatter, "{}: {}", self.kind, self.message),
        }
    }
}

impl std::error::Error for ServerPropertiesError {}

#[cfg(test)]
mod tests {
    use next_web_core::{
        env::{BaseEnvironment, ConfigurableEnvironment, MapPropertySource},
        util::indexmap::IndexMap,
    };

    use super::*;

    /// Creates an environment holding the given properties.
    fn environment(properties: &[(&str, &str)]) -> BaseEnvironment {
        let mut environment = BaseEnvironment::new();
        environment
            .property_sources()
            .add_last(Box::new(MapPropertySource::new(
                "test".to_owned(),
                properties
                    .iter()
                    .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                    .collect::<IndexMap<_, _>>(),
            )));

        environment
    }

    #[test]
    fn defaults_describe_the_server_of_an_application_that_configures_nothing() {
        let properties = ServerProperties::default();

        assert_eq!(properties.port(), DEFAULT_PORT);
        assert_eq!(properties.address(), None);
        assert_eq!(properties.bind_address(), DEFAULT_ADDRESS);
        assert_eq!(properties.context_path(), None);
        assert_eq!(properties.http().request().timeout(), 5);
        assert!(!properties.http().request().trace());
        assert!(!properties.ssl().is_enabled());
        assert_eq!(
            properties.socket_addr().map(|address| address.to_string()),
            Ok(format!("{DEFAULT_ADDRESS}:{DEFAULT_PORT}"))
        );
    }

    #[test]
    fn a_document_is_deserialized_with_the_defaults_of_the_fields_it_omits() {
        let properties: ServerProperties = serde_yaml::from_str(
            r#"
            port: 8080
            context_path: /app
            http:
              request:
                max_request_size: 1048576
                timeout: 30
            ssl:
              enabled: true
            "#,
        )
        .expect("the document describes the properties of a server");

        assert_eq!(properties.port(), 8080);
        assert_eq!(properties.address(), None);
        assert_eq!(properties.bind_address(), DEFAULT_ADDRESS);
        assert_eq!(properties.context_path(), Some("/app"));
        assert_eq!(
            properties.http().request().max_request_size(),
            Some(1_048_576)
        );
        assert_eq!(properties.http().request().timeout(), 30);
        assert!(!properties.http().request().trace());
        assert!(properties.ssl().is_enabled());
    }

    #[test]
    fn the_properties_are_serialized_back_to_the_document_they_were_read_from() {
        let properties: ServerProperties =
            serde_yaml::from_str("port: 8080\naddress: 127.0.0.1\n").expect("the document is read");

        let document = serde_yaml::to_string(&properties).expect("the properties are written");
        let written: ServerProperties =
            serde_yaml::from_str(&document).expect("what was written is read again");

        assert_eq!(written, properties);
    }

    #[test]
    fn the_properties_are_bound_from_the_environment() {
        let environment = environment(&[
            ("next.server.port", "9090"),
            ("next.server.address", "127.0.0.1"),
            ("next.server.context_path", "app/"),
            ("next.server.http.request.timeout", "15"),
            ("next.server.ssl.enabled", "true"),
        ]);

        let properties =
            ServerProperties::from_environment(&environment).expect("the properties bind");

        assert_eq!(properties.port(), 9090);
        assert_eq!(properties.address(), Some("127.0.0.1"));
        assert_eq!(
            properties.normalized_context_path().as_deref(),
            Some("/app")
        );
        assert_eq!(properties.http().request().timeout(), 15);
        assert!(properties.ssl().is_enabled());
    }

    #[test]
    fn the_defaults_are_bound_when_the_environment_configures_no_server() {
        let environment = environment(&[("next.application.name", "demo")]);

        let properties =
            ServerProperties::from_environment(&environment).expect("the properties bind");

        assert_eq!(properties, ServerProperties::default());
    }

    #[test]
    fn a_property_that_is_not_a_value_is_reported_with_its_key() {
        let environment = environment(&[("next.server.port", "not-a-port")]);

        let error = ServerProperties::from_environment(&environment)
            .expect_err("the port of the environment is not a number");

        assert_eq!(error.kind(), ServerPropertiesErrorKind::InvalidValue);
        assert_eq!(error.key(), Some("next.server.port"));
        assert!(error.message().contains("invalid"));
    }

    #[test]
    fn an_address_that_a_socket_cannot_hold_is_reported() {
        let properties = ServerProperties::new(8080, "not-an-address", "");

        let error = properties
            .socket_addr()
            .expect_err("the address is not an IP");

        assert_eq!(error.kind(), ServerPropertiesErrorKind::InvalidAddress);
        assert_eq!(error.key(), Some("not-an-address:8080"));
        assert!(error
            .to_string()
            .starts_with(ServerPropertiesErrorKind::InvalidAddress.description()));
        assert!(error.to_string().contains("not-an-address:8080"));
    }

    #[test]
    fn a_context_path_is_normalized_to_the_path_the_routes_are_nested_under() {
        assert_eq!(
            ServerProperties::normalize_context_path("app").as_deref(),
            Some("/app")
        );
        assert_eq!(
            ServerProperties::normalize_context_path(" /app/ ").as_deref(),
            Some("/app")
        );
        assert_eq!(
            ServerProperties::normalize_context_path("/app/rest").as_deref(),
            Some("/app/rest")
        );
        assert_eq!(ServerProperties::normalize_context_path("/"), None);
        assert_eq!(ServerProperties::normalize_context_path("   "), None);
        assert_eq!(ServerProperties::normalize_context_path(""), None);
    }

    #[test]
    fn the_properties_of_the_process_are_read_once_they_are_installed() {
        let _installed = ServerProperties {
            port: 7070,
            ..ServerProperties::default()
        }
        .install_global();

        // Another test of the process may have installed the properties first,
        // in which case the installation reports it and the properties that
        // were installed are returned instead.
        assert!(ServerProperties::global().is_some());
    }
}
