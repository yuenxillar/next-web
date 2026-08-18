use crate::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

/// Handler for CORS pre-flight requests.
pub trait PreFlightRequestHandler
where
    Self: Send + Sync,
{
    /// Handles a pre-flight request by finding and applying the CORS configuration
    /// that matches the expected actual request. As a result of handling, the
    /// response should be updated with CORS headers or rejected.
    fn handle_pre_flight(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError>;
}
