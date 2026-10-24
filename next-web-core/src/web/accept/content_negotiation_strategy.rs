use crate::{error::BoxError, http::HttpRequest, http::MediaType};

/// A strategy for resolving the requested media types for a request.
pub trait ContentNegotiationStrategy
where
    Self: Send + Sync,
{
    fn resolve_media_types(&self, request: &dyn HttpRequest) -> Result<Vec<MediaType>, BoxError>;
}
