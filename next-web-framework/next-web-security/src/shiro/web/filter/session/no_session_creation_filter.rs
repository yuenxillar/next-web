use std::ops::{Deref, DerefMut};

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
        required::Required,
    },
};

use crate::{
    core::{
        subject::support::default_subject_context::DefaultSubjectContext, util::object::Object,
    },
    web::filter::{
        advice_filter::AdviceFilterExt,
        once_per_request_filter::OncePerRequestFilter,
        path_matching_filter::{PathMatchingFilter, PathMatchingFilterExt},
    },
};

#[derive(Clone)]
pub struct NoSessionCreationFilter {
    path_matching_filter: PathMatchingFilter,
}

#[async_trait]
impl PathMatchingFilterExt for NoSessionCreationFilter {
    #[allow(unused_variables)]
    async fn on_pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        request.set_attribute(
            DefaultSubjectContext::SESSION_CREATION_ENABLED,
            AnyValue::Boolean(false),
        );
        true
    }
}

#[async_trait]
impl AdviceFilterExt for NoSessionCreationFilter {}

impl Required<OncePerRequestFilter> for NoSessionCreationFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }
}

impl Named for NoSessionCreationFilter {
    fn name(&self) -> &str {
        "NoSessionCreationFilter"
    }
}

impl Deref for NoSessionCreationFilter {
    type Target = PathMatchingFilter;

    fn deref(&self) -> &Self::Target {
        &self.path_matching_filter
    }
}

impl DerefMut for NoSessionCreationFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.path_matching_filter
    }
}

impl Default for NoSessionCreationFilter {
    fn default() -> Self {
        Self {
            path_matching_filter: Default::default(),
        }
    }
}
