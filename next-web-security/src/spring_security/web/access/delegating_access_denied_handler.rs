use std::{collections::BTreeMap, sync::Arc};

use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::web::access::AccessDeniedHandler;

pub struct DelegatingAccessDeniedHandler {
    handlers: BTreeMap<&'static str, Arc<dyn AccessDeniedHandler>>,
    default_handler: Arc<dyn AccessDeniedHandler>,
}

impl DelegatingAccessDeniedHandler {
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
        access_denied_error: super::AccessDeniedError,
    ) -> Result<(), BoxError> {
        todo!()
    }
}
