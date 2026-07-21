use crate::{
    error::BoxError, http::MediaType, traits::http::http_request::HttpRequest,
    web::accept::ContentNegotiationStrategy,
};

#[derive(Clone, Default)]
pub struct HeaderContentNegotiationStrategy;

impl ContentNegotiationStrategy for HeaderContentNegotiationStrategy {
    /// Resolve the given request to a list of media types. The returned list is ordered by specificity first and by quality parameter second.
    fn resolve_media_types(&self, request: &dyn HttpRequest) -> Result<Vec<MediaType>, BoxError> {
        todo!()
    }
}
