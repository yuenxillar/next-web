use axum::http::StatusCode;
use next_web_core::error::BoxError;
use tracing::debug;

use super::{request_rejectedError::RequestRejectedError, request_rejected_handler::RequestRejectedHandler};

#[derive(Clone)]
pub struct HttpStatusRequestRejectedHandler {
    pub http_status: StatusCode,
}

impl Default for HttpStatusRequestRejectedHandler {
    fn default() -> Self {
        Self {
            http_status: StatusCode::BAD_REQUEST,
          }
    }
}

impl RequestRejectedHandler for HttpStatusRequestRejectedHandler {
    fn handle(
        &self,
        _request: &mut axum::extract::Request,
        response: &mut axum::response::Response,
        request_rejected_error: & RequestRejectedError,
    ) -> Result<(), BoxError> {
        debug!("Rejecting request due to: {}", request_rejected_error.0);

        *response.status_mut() = self.http_status;
        Ok(())
    }
}