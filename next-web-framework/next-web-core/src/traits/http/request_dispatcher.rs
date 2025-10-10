use axum::{extract::Request, response::Response};

use crate::error::BoxError;



pub trait RequestDispatcher {
    
    fn forward(&self, request: & Request, response: &mut Response) -> Result<(), BoxError>;

    fn include(&self, request: & Request, response: &mut Response) -> Result<(), BoxError>;
}