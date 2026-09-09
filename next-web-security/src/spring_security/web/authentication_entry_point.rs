use std::sync::Arc;

use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::core::AuthenticationError;

pub trait AuthenticationEntryPoint
where
    Self: Send + Sync,
{
    fn commence(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        auth_error: &AuthenticationError,
    ) -> Result<(), BoxError>;
}

impl<F> AuthenticationEntryPoint for F
where
    F: Fn(
            &mut dyn HttpRequest,
            &mut dyn HttpResponse,
            &AuthenticationError,
        ) -> Result<(), BoxError>
        + Send
        + Sync
        + 'static,
{
    fn commence(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        auth_error: &AuthenticationError,
    ) -> Result<(), BoxError> {
        self(request, response, auth_error)
    }
}

/// Wraps a function as an `AuthenticationEntryPoint`.
struct AuthenticationEntryPointFnWrapper<F>(pub F);

impl<F> AuthenticationEntryPoint for AuthenticationEntryPointFnWrapper<F>
where
    F: Fn(
            &mut dyn HttpRequest,
            &mut dyn HttpResponse,
            &AuthenticationError,
        ) -> Result<(), BoxError>
        + Send
        + Sync
        + 'static,
{
    fn commence(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        auth_error: &AuthenticationError,
    ) -> Result<(), BoxError> {
        (self.0)(request, response, auth_error)
    }
}

pub fn authentication_entry_point_fn_wrapper<F>(func: F) -> Arc<dyn AuthenticationEntryPoint>
where
    F: Fn(
            &mut dyn HttpRequest,
            &mut dyn HttpResponse,
            &AuthenticationError,
        ) -> Result<(), BoxError>
        + Send
        + Sync
        + 'static,
{
    Arc::new(AuthenticationEntryPointFnWrapper(func))
}
