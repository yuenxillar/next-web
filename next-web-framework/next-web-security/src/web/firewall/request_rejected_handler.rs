use axum::{extract::Request, response::Response};
use next_web_core::error::BoxError;

use super::request_rejectedError::RequestRejectedError;

pub trait RequestRejectedHandler
where
    Self: Send + Sync,
{
    fn handle(
        &self,
        request: &mut Request,
        response: &mut Response,
        request_rejected_error: & RequestRejectedError,
    ) -> Result<(), BoxError>;
}
