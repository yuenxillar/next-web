use std::sync::Arc;

use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::access::AccessDeniedError;

pub trait AccessDeniedHandler: Send + Sync {
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        access_denied_error: &AccessDeniedError,
    ) -> Result<(), BoxError>;
}

impl<F> AccessDeniedHandler for F
where
    F: Fn(&mut dyn HttpRequest, &mut dyn HttpResponse, &AccessDeniedError) -> Result<(), BoxError>
        + Send
        + Sync
        + 'static,
{
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        access_denied_error: &AccessDeniedError,
    ) -> Result<(), BoxError> {
        self(request, response, access_denied_error)
    }
}

/// Wraps a function as an `AccessDeniedHandler`.
struct AccessDeniedHandlerFnWrapper<F>(pub F);

impl<F> AccessDeniedHandler for AccessDeniedHandlerFnWrapper<F>
where
    F: Fn(&mut dyn HttpRequest, &mut dyn HttpResponse, &AccessDeniedError) -> Result<(), BoxError>
        + Send
        + Sync
        + 'static,
{
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        access_denied_error: &AccessDeniedError,
    ) -> Result<(), BoxError> {
        (self.0)(request, response, access_denied_error)
    }
}

pub fn access_denied_handler_fn_wrapper<F>(func: F) -> Arc<dyn AccessDeniedHandler>
where
    F: Fn(&mut dyn HttpRequest, &mut dyn HttpResponse, &AccessDeniedError) -> Result<(), BoxError>
        + Send
        + Sync
        + 'static,
{
    Arc::new(AccessDeniedHandlerFnWrapper(func))
}
