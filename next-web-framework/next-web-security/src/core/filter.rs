use axum::{extract::Request,response::Response};
use next_web_core::error::BoxError;

pub trait Filter: Send + Sync {
    
    fn do_filter(&self, req: &mut Request, res: &mut Response) -> Result<(), BoxError>;
}