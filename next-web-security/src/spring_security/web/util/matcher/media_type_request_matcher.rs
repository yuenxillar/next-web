use std::{collections::HashSet, fmt::Debug, sync::Arc};

use next_web_core::{
    http::MediaType,
    traits::http::http_request::HttpRequest,
    web::accept::{ContentNegotiationStrategy, HeaderContentNegotiationStrategy},
};

use crate::web::util::matcher::RequestMatcher;

#[derive(Clone)]
pub struct MediaTypeRequestMatcher {
    content_negotiation_strategy: Arc<dyn ContentNegotiationStrategy>,
    matching_media_types: Vec<MediaType>,
    use_equals: bool,
    ignored_media_types: HashSet<MediaType>,
}

impl MediaTypeRequestMatcher {
    /// Creates an instance with a `HeaderContentNegotiationStrategy` as the default
    /// content negotiation strategy.
    ///
    /// # Arguments
    /// * `matching_media_types` - the `MediaType`s that will cause the matcher to return true
    ///
    /// # Panics
    /// Panics if `matching_media_types` is empty.
    pub fn new(matching_media_types: Vec<MediaType>) -> Self {
        Self::with_strategy(
            Arc::new(HeaderContentNegotiationStrategy::default()),
            matching_media_types,
        )
    }

    /// Creates an instance with a specified content negotiation strategy.
    ///
    /// # Arguments
    /// * `content_negotiation_strategy` - the `ContentNegotiationStrategy` to use
    /// * `matching_media_types` - the `MediaType`s that will cause the matcher to return true
    ///
    /// # Panics
    /// Panics if `matching_media_types` is empty.
    pub fn with_strategy(
        content_negotiation_strategy: Arc<dyn ContentNegotiationStrategy>,
        matching_media_types: Vec<MediaType>,
    ) -> Self {
        assert!(
            !matching_media_types.is_empty(),
            "matchingMediaTypes cannot be null or empty"
        );
        Self {
            content_negotiation_strategy,
            matching_media_types,
            use_equals: false,
            ignored_media_types: HashSet::new(),
        }
    }

    /// Checks if the given media type should be ignored based on the configured ignored types.
    ///
    /// # Arguments
    /// * `http_request_media_type` - the media type to check
    ///
    /// # Returns
    /// `true` if the media type should be ignored, `false` otherwise
    fn should_ignore(&self, http_request_media_type: &MediaType) -> bool {
        for ignored_media_type in self.ignored_media_types.iter() {
            if http_request_media_type.includes(Some(ignored_media_type)) {
                return true;
            }
        }
        false
    }

    /// If set to true, matches on exact `MediaType`, else uses
    /// `MediaType::is_compatible_with`.
    ///
    /// # Arguments
    /// * `use_equals` - specify if equals comparison should be used
    pub fn set_use_equals(&mut self, use_equals: bool) {
        self.use_equals = use_equals;
    }

    /// Returns whether exact matching is enabled.
    ///
    /// # Returns
    /// `true` if exact matching is used, `false` if compatible matching is used
    pub fn is_use_equals(&self) -> bool {
        self.use_equals
    }

    /// Set the `MediaType`s to ignore from the `ContentNegotiationStrategy`.
    /// This is useful if for example, you want to match on
    /// `MediaType::APPLICATION_JSON` but want to ignore `MediaType::ALL`.
    ///
    /// # Arguments
    /// * `ignored_media_types` - the `MediaType`s to ignore from the
    ///   `ContentNegotiationStrategy`
    pub fn set_ignored_media_types(&mut self, ignored_media_types: Vec<MediaType>) {
        self.ignored_media_types = ignored_media_types.into_iter().collect();
    }
}

impl RequestMatcher for MediaTypeRequestMatcher {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        let http_request_media_types = match self
            .content_negotiation_strategy
            .resolve_media_types(request)
        {
            Ok(types) => types,
            Err(err) => {
                tracing::debug!(
                    "Failed to match request since failed to parse MediaTypes: {}",
                    err
                );
                return false;
            }
        };

        for http_request_media_type in http_request_media_types.iter() {
            if self.should_ignore(http_request_media_type) {
                continue;
            }

            if self.use_equals {
                return self.matching_media_types.contains(http_request_media_type);
            }

            for matching_media_type in self.matching_media_types.iter() {
                if matching_media_type.is_compatible_with(Some(http_request_media_type)) {
                    return true;
                }
            }
        }

        false
    }
}

impl Debug for MediaTypeRequestMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MediaTypeRequestMatcher")
    }
}
