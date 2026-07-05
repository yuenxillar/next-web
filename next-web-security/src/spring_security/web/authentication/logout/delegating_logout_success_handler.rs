use std::sync::Arc;

use next_web_core::{
    async_trait,
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::Authentication,
    web::{authentication::logout::LogoutSuccessHandler, util::matcher::RequestMatcher},
};

/// Delegates to logout handlers based on matched request matchers
#[derive(Clone)]
pub struct DelegatingLogoutSuccessHandler {
    matcher_to_handler: Vec<(Arc<dyn RequestMatcher>, Arc<dyn LogoutSuccessHandler>)>,
    default_logout_success_handler: Option<Arc<dyn LogoutSuccessHandler>>,
}

impl DelegatingLogoutSuccessHandler {
    pub fn new(
        default_logout_success_handlers: Vec<(
            Arc<dyn RequestMatcher>,
            Arc<dyn LogoutSuccessHandler>,
        )>,
    ) -> Self {
        Self {
            matcher_to_handler: default_logout_success_handlers,
            default_logout_success_handler: None,
        }
    }

    /// Sets the default LogoutSuccessHandler if no other handlers available
    pub fn set_default_logout_success_handler(&mut self, handler: Arc<dyn LogoutSuccessHandler>) {
        self.default_logout_success_handler = Some(handler);
    }
}

#[async_trait]
impl LogoutSuccessHandler for DelegatingLogoutSuccessHandler {
    async fn on_logout_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Option<&dyn Authentication>,
    ) -> Result<(), BoxError> {
        for (matcher, handler) in self.matcher_to_handler.iter() {
            if matcher.matches(request) {
                handler
                    .on_logout_success(request, response, authentication)
                    .await?;
                return Ok(());
            }
        }

        if let Some(handler) = self.default_logout_success_handler.as_ref() {
            handler
                .on_logout_success(request, response, authentication)
                .await?;
        }

        Ok(())
    }
}
