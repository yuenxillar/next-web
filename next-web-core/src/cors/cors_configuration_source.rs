use crate::{cors::CorsConfiguration, traits::http::http_request::HttpRequest};

pub trait CorsConfigurationSource
where
    Self: Send + Sync,
{
    fn cors_configuration(&self, request: &dyn HttpRequest) -> Option<&CorsConfiguration>;
}
