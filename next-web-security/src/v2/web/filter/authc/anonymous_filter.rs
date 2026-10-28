use next_web_core::{
    Named, Required, async_trait,
    http::{HttpRequest, HttpResponse},
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
impl AdviceFilterExt for AnonymousFilter {
    async fn pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        ext: Option<&dyn PathMatchingFilterExt>,
    ) -> bool {
        <Self as Required<PathMatchingFilter>>::get_object(self)
            .pre_handle(request, response, ext)
            .await
    }
}

impl Required<OncePerRequestFilter> for AnonymousFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self.path_matching_filter.once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self.path_matching_filter.once_per_request_filter
    }
}

impl Required<PathMatchingFilter> for AnonymousFilter {
    fn get_object(&self) -> &PathMatchingFilter {
        &self.path_matching_filter
    }

    fn get_mut_object(&mut self) -> &mut PathMatchingFilter {
        &mut self.path_matching_filter
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
