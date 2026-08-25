use std::sync::Arc;

use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    access::AccessDeniedError,
    web::{access::AccessDeniedHandler, util::matcher::RequestMatcher},
};

/// An AccessDeniedHandler that delegates to other AccessDeniedHandler instances based upon the
/// type of HttpRequest passed into handle(HttpRequest, HttptResponse, AccessDeniedError)
pub struct RequestMatcherDelegatingAccessDeniedHandler {
    handlers: Vec<(Arc<dyn RequestMatcher>, Arc<dyn AccessDeniedHandler>)>,
    default_handler: Arc<dyn AccessDeniedHandler>,
}

impl RequestMatcherDelegatingAccessDeniedHandler {
    /// Creates a new instance
    pub fn new(
        handlers: Vec<(Arc<dyn RequestMatcher>, Arc<dyn AccessDeniedHandler>)>,
        default_handler: Arc<dyn AccessDeniedHandler>,
    ) -> Self {
        Self {
            handlers,
            default_handler,
        }
    }
}

impl AccessDeniedHandler for RequestMatcherDelegatingAccessDeniedHandler {
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        access_denied_error: &AccessDeniedError,
    ) -> Result<(), BoxError> {
        for (matcher, handler) in self.handlers.iter() {
            if matcher.matches(request) {
                return handler.handle(request, response, access_denied_error);
            }
        }

        self.default_handler
            .handle(request, response, access_denied_error)
    }
}
