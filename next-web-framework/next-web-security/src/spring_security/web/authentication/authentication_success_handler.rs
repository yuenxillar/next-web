use axum::{extract::Request, response::Response};

use crate::core::authentication::Authentication;



pub trait AuthenticationSuccessHandler: Send + Sync {
    fn on_authentication_success(&self, request: &Request, response: &mut Response, authentication: &dyn Authentication);
}