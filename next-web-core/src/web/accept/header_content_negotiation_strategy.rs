use crate::{
    error::BoxError, http::MediaType, traits::http::http_request::HttpRequest, util::MimeTypeUtils,
    web::accept::ContentNegotiationStrategy,
};

/// A ContentNegotiationStrategy that checks the 'Accept' request header.
#[derive(Clone, Default)]
pub struct HeaderContentNegotiationStrategy;

impl ContentNegotiationStrategy for HeaderContentNegotiationStrategy {
    /// Resolve the given request to a list of media types. The returned list is ordered by specificity first and by quality parameter second.
    fn resolve_media_types(&self, request: &dyn HttpRequest) -> Result<Vec<MediaType>, BoxError> {
        let header_values = request.header_values("accept");

        if header_values.is_empty() {
            return Ok(MediaType::media_type_all_list());
        }

        let mut media_types = MediaType::parse_media_types_from_str_list(Some(&header_values));
        MimeTypeUtils::sort_by_specificity(&mut media_types)
            .map_err(|err| Into::<BoxError>::into(err))?;
        if media_types.is_empty() {
            return Ok(MediaType::media_type_all_list());
        }

        Ok(media_types)
    }
}
