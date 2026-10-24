use next_web_core::http::StatusCode;
use next_web_core::{
    error::BoxError,
   http::{HttpRequest, HttpResponse},
};
use tracing::debug;

use super::{
    request_rejected_error::RequestRejectedError, request_rejected_handler::RequestRejectedHandler,
};

/// A simple implementation of RequestRejectedHandler that sends an error with configurable status code.
#[derive(Clone)]
pub struct HttpStatusRequestRejectedHandler {
    pub status_code: StatusCode,
}

impl HttpStatusRequestRejectedHandler {
    /// Constructs an instance which uses a configurable http code as response.
    pub fn new(status_code: StatusCode) -> Self {
        Self { status_code }
    }
}
impl Default for HttpStatusRequestRejectedHandler {
    /// Constructs an instance which uses 400 as response code.
    fn default() -> Self {
        Self {
            status_code: StatusCode::BAD_REQUEST,
        }
    }
}

impl RequestRejectedHandler for HttpStatusRequestRejectedHandler {
    fn handle(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        request_rejected_error: &RequestRejectedError,
    ) -> Result<(), BoxError> {
        debug!("Rejecting request due to: {}", request_rejected_error.0);

        response.set_status_code(self.status_code);
        Ok(())
    }
}
