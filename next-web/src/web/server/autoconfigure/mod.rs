//! The configuration of the web server of an application.
//!
//! The properties of the server are the properties a server binds from the
//! `next.server` prefix of the environment of the application:
//!
//! - [`ServerProperties`] holds where the server listens and what it serves,
//! - [`HttpProperties`] holds the settings of the requests and the responses,
//! - [`SslProperties`] holds the TLS configuration of the server.
//!
//! The modules are re-exported from here, so an application imports the
//! properties it needs from one place, and the singleton of the application
//! context holds them under the name `serverProperties`.

mod http_properties;
mod server_properties;
mod ssl_properties;

pub use http_properties::{
    HttpProperties, RequestProperties, ResponseProperties, DEFAULT_REQUEST_TIMEOUT_SECONDS,
};
pub use server_properties::{
    ServerProperties, ServerPropertiesError, ServerPropertiesErrorKind, DEFAULT_ADDRESS,
    DEFAULT_PORT, GLOBAL_SERVER_PROPERTIES, SERVER_PROPERTIES_PREFIX,
};
pub use ssl_properties::SslProperties;
