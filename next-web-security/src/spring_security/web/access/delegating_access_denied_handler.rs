use std::{collections::BTreeMap, sync::Arc};

use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::access::AccessDeniedError;
use crate::web::access::AccessDeniedHandler;

/// An AccessDeniedHandler that delegates to other AccessDeniedHandler instances based upon the type of
/// AccessDeniedError passed into handle(HttpRequest, HttpResponse, AccessDeniedError).
pub struct DelegatingAccessDeniedHandler {
    handlers: BTreeMap<&'static str, Arc<dyn AccessDeniedHandler>>,
    default_handler: Arc<dyn AccessDeniedHandler>,
}

impl DelegatingAccessDeniedHandler {
    /// Creates a new instance
    pub fn new(
        handlers: BTreeMap<&'static str, Arc<dyn AccessDeniedHandler>>,
        default_handler: Arc<dyn AccessDeniedHandler>,
    ) -> Self {
        Self {
            handlers,
            default_handler,
        }
    }
}

impl AccessDeniedHandler for DelegatingAccessDeniedHandler {
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        access_denied_error: &AccessDeniedError,
    ) -> Result<(), BoxError> {
        for (handler_type, handler) in self.handlers.iter() {
            if *handler_type == access_denied_error.type_name() {
                return handler.handle(request, response, access_denied_error);
            }
        }

        self.default_handler
            .handle(request, response, access_denied_error)
    }
}
