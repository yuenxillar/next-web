use crate::{cors::CorsConfiguration, traits::http::http_request::HttpRequest};

/// Interface to be implemented by classes (usually HTTP request handlers) that provides a
/// CorsConfiguration instance based on the provided request.
pub trait CorsConfigurationSource
where
    Self: Send + Sync,
{
    /// Return a CorsConfiguration based on the incoming request.
    fn cors_configuration(&self, request: &dyn HttpRequest) -> Option<&CorsConfiguration>;
}
