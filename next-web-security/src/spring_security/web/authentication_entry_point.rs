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
