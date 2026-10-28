use next_web_core::{
    error::BoxError,
    http::{HttpRequest, HttpResponse},
};

use super::request_rejected_error::RequestRejectedError;

pub trait RequestRejectedHandler
where
    Self: Send + Sync,
{
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        request_rejected_error: &RequestRejectedError,
    ) -> Result<(), BoxError>;
}
