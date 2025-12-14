use axum::{extract::Request, response::Response};
use next_web_core::error::BoxError;

use crate::core::authentication_error::AuthenticationError;

pub trait AuthenticationEntryPoint: Send + Sync {
    fn commence(
        &self,
        request: &mut Request,
        response: &mut Response,
        auth_error: Option<AuthenticationError>,
    ) -> Result<(), BoxError>;
}
