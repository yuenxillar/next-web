use next_web_core::{
    async_trait,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
        required::Required,
    },
};

use crate::{
    core::util::object::Object,
    web::filter::{
        advice_filter::AdviceFilterExt,
        once_per_request_filter::OncePerRequestFilter,
        path_matching_filter::{PathMatchingFilter, PathMatchingFilterExt},
    },
};

#[derive(Clone, Default)]
pub struct AnonymousFilter {
    pub(crate) path_matching_filter: PathMatchingFilter,
}

#[async_trait]
impl AdviceFilterExt for AnonymousFilter {}

impl Required<OncePerRequestFilter> for AnonymousFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self.path_matching_filter.once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self.path_matching_filter.once_per_request_filter
    }
}

impl Named for AnonymousFilter {
    fn name(&self) -> &str {
        "AnonymousFilter"
    }
}

#[async_trait]
impl PathMatchingFilterExt for AnonymousFilter {
    #[allow(unused_variables)]
    async fn on_pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        // Always return true since we allow access to anyone
        true
    }
}
