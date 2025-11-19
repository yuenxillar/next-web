use std::ops::{Deref, DerefMut};

use next_web_core::{
    async_trait,
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::web::filter::{once_per_request_filter::OncePerRequestFilter, path_matching_filter::PathMatchingFilterExt};

#[derive(Clone)]
pub struct AdviceFilter {
    pub(crate) once_per_request_filter: OncePerRequestFilter,
}

impl Deref for AdviceFilter {
    type Target = OncePerRequestFilter;

    fn deref(&self) -> &Self::Target {
        &self.once_per_request_filter
    }
}

impl DerefMut for AdviceFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.once_per_request_filter
    }
}

impl Default for AdviceFilter {
    fn default() -> Self {
        Self {
            once_per_request_filter: Default::default(),
        }
    }
}

#[allow(unused_variables)]
#[async_trait]
pub trait AdviceFilterExt: Send + Sync {
    async fn pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        ext: Option<&dyn PathMatchingFilterExt>,
    ) -> bool {
        true
    }

    async fn post_handle(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        Ok(())
    }

    async fn cleanup(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        error: Option<BoxError>,
    ) -> Result<(), BoxError> {
        Ok(())
    }
}
