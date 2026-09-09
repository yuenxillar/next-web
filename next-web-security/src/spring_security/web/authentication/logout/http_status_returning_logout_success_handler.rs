use std::sync::Arc;

use next_web_core::{
    async_trait,
    error::BoxError,
    http::StatusCode,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{core::Authentication, web::authentication::logout::LogoutSuccessHandler};

/// Implementation of the LogoutSuccessHandler. By default returns an HTTP status code of 200. This is
/// useful in REST-type scenarios where a redirect upon a successful logout is not desired.
#[derive(Clone)]
pub struct HttpStatusReturningLogoutSuccessHandler {
    http_status_to_return: StatusCode,
}

impl HttpStatusReturningLogoutSuccessHandler {
    pub fn new(http_status_to_return: StatusCode) -> Self {
        Self {
            http_status_to_return,
        }
    }
}

/// Implementation of LogoutSuccessHandler.onLogoutSuccess(HttpServletRequest, HttpServletResponse, Authentication)
#[async_trait]
impl LogoutSuccessHandler for HttpStatusReturningLogoutSuccessHandler {
    async fn on_logout_success(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _authentication: Option<&Arc<dyn Authentication>>,
    ) -> Result<(), BoxError> {
        response.set_status_code(self.http_status_to_return);
        response.commit();

        Ok(())
    }
}

impl Default for HttpStatusReturningLogoutSuccessHandler {
    /// HttpStatusLogoutSuccessHandler with the default HttpStatus.OK.
    fn default() -> Self {
        Self::new(StatusCode::OK)
    }
}
