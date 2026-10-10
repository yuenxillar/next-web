pub mod autoconfigure;

mod filter_layer;
mod request_layer;
mod server;
#[cfg(feature = "tls-rustls")]
mod tls_web_server;
mod web_server;

pub(crate) use filter_layer::add_filter_layer;
pub(crate) use request_layer::prepare_request_layer;
pub use server::Server;
#[cfg(feature = "tls-rustls")]
pub use tls_web_server::{
    TlsWebServer, TlsWebServerError, SSL_CERTIFICATE_PRIVATE_KEY_PROPERTY,
    SSL_CERTIFICATE_PROPERTY, SSL_ENABLED_PROPERTY,
};
pub use web_server::WebServer;
