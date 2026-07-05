use std::{collections::BTreeMap, sync::Arc};

use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::web::access::{AccessDeniedError, AccessDeniedHandler};

pub struct RequestMatcherDelegatingAccessDeniedHandler {
    handlers: BTreeMap<String, Arc<dyn AccessDeniedHandler>>,
    default_handler: Arc<dyn AccessDeniedHandler>,
}

impl RequestMatcherDelegatingAccessDeniedHandler {
    pub fn new(
        handlers: BTreeMap<String, Arc<dyn AccessDeniedHandler>>,
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
        access_denied_error: AccessDeniedError,
    ) -> Result<(), BoxError> {
        todo!()
    }
}
