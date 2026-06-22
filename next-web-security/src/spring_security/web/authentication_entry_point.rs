use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::core::authentication_error::AuthenticationError;

pub trait AuthenticationEntryPoint: Send + Sync {
    fn commence(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        auth_error: Option<AuthenticationError>,
    ) -> Result<(), BoxError>;
}
