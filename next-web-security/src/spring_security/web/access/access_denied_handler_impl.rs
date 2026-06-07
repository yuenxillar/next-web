use next_web_core::{error::BoxError, traits::http::{http_request::HttpRequest, http_response::HttpResponse}};

use crate::web::access::{AccessDeniedError, AccessDeniedHandler};

#[derive(Default)]
pub struct AccessDeniedHandlerImpl {}

impl AccessDeniedHandler for AccessDeniedHandlerImpl {
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        access_denied_error: AccessDeniedError,
    ) -> Result<(), BoxError> {
        todo!()
    }
}
