use std::{collections::HashSet, sync::Arc};

use next_web_core::{
    traits::http::http_request::HttpRequest, web::accept::ContentNegotiationStrategy,
};

use crate::web::util::matcher::RequestMatcher;

#[derive(Debug, Clone)]
pub struct MediaTypeRequestMatcher {
    content_negotiation_strategy: Arc<dyn ContentNegotiationStrategy>,
    ignored_media_types: HashSet<MediaType>,
}

impl MediaTypeRequestMatcher {
    pub fn new(
        content_negotiation_strategy: Arc<dyn ContentNegotiationStrategy>,
        matching_media_types: Vec<MediaType>,
    ) -> Self {
        Self {
            content_negotiation_strategy,
            ignored_media_types: matching_media_types.into(),
        }
    }

    pub fn set_ignored_media_types(&mut self, ignored_media_types: Vec<MediaType>) {
        self.ignored_media_types = ignored_media_types.into_iter().collect();
    }
}

impl RequestMatcher for MediaTypeRequestMatcher {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        todo!()
    }
}
