use next_web_core::http::StatusCode;
use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::debug;

use super::{
    request_rejected_error::RequestRejectedError, request_rejected_handler::RequestRejectedHandler,
};

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
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        request_rejected_error: &RequestRejectedError,
    ) -> Result<(), BoxError> {
        debug!("Rejecting request due to: {}", request_rejected_error.0);

        response.set_status_code(self.http_status);
        Ok(())
    }
}
