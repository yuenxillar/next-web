use std::sync::Arc;

use axum::BoxError;
use indexmap::IndexMap;
use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::access::AccessDeniedHandler;

pub struct DelegatingAccessDeniedHandler {
    handlers: IndexMap<&'static str, Arc<dyn AccessDeniedHandler>>,
    default_handler: Arc<dyn AccessDeniedHandler>,
}

impl DelegatingAccessDeniedHandler {
    pub fn new(
        handlers: IndexMap<&'static str, Arc<dyn AccessDeniedHandler>>,
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
        access_denied_error: super::AccessDeniedError,
    ) -> Result<(), BoxError> {
        todo!()
    }
}
