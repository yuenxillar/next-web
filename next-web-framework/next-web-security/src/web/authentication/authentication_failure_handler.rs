use axum::{extract::Request, response::Response};

use crate::core::authentication_error::AuthenticationError;


pub trait AuthenticationFailureHandler: Send + Sync {
    
    fn on_authentication_failure(&self, request: &Request, response: &mut  Response,  error: & AuthenticationError);
}