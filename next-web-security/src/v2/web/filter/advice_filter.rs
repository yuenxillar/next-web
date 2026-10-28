use std::ops::{Deref, DerefMut};

use next_web_core::{
    async_trait,
    error::BoxError,
    http::{HttpRequest, HttpResponse},
};

use crate::web::filter::{
    once_per_request_filter::OncePerRequestFilter, path_matching_filter::PathMatchingFilterExt,
};

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
        match error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Applies the login URL of the application to this filter.
    ///
    /// The default keeps the login URL the filter was built with and reports
    /// `false`; a filter that answers a login page takes the URL of the
    /// application and reports `true`.
    ///
    /// # Arguments
    ///
    /// * `login_url` - The path of the login page of the application.
    fn apply_login_url(&mut self, _login_url: &str) -> bool {
        false
    }

    /// Applies the success URL of the application to this filter.
    ///
    /// The default keeps the URL the filter was built with and reports `false`;
    /// a filter that answers a successful login takes the URL of the
    /// application and reports `true`.
    ///
    /// # Arguments
    ///
    /// * `success_url` - The path of the page a successful login returns to.
    fn apply_success_url(&mut self, _success_url: &str) -> bool {
        false
    }

    /// Applies the unauthorized URL of the application to this filter.
    ///
    /// The default keeps the URL the filter was built with and reports `false`;
    /// a filter that refuses a request takes the URL of the application and
    /// reports `true`.
    ///
    /// # Arguments
    ///
    /// * `unauthorized_url` - The path of the page a refused request returns to.
    fn apply_unauthorized_url(&mut self, _unauthorized_url: &str) -> bool {
        false
    }
}
