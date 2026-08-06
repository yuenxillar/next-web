use next_web_core::{
    error::BoxError,
    http::StatusCode,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::debug;

use crate::{core::authentication_error::AuthenticationError, web::AuthenticationEntryPoint};

/// In the pre-authenticated authentication case (unlike CAS, for example) the user will already have been
/// identified through some external mechanism and a secure context established by the time the
/// security-enforcement filter is invoked.
///
/// Therefore this class isn't actually responsible for the commencement of authentication, as it is in the
/// case of other providers. It will be called if the user is rejected by the BasePreAuthenticatedProcessingFilter, resulting in a none authentication.
/// The commence method will always return an HttpServletResponse.SC_FORBIDDEN (403 error).
#[derive(Clone, Default)]
pub struct Http403ForbiddenEntryPoint;

impl AuthenticationEntryPoint for Http403ForbiddenEntryPoint {
    /// Always returns a 403 error code to the client.
    fn commence(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _auth_error: &AuthenticationError,
    ) -> Result<(), BoxError> {
        debug!("Pre-authenticated entry point called. Rejecting access");
        response.set_status_code(StatusCode::FORBIDDEN);
        response.set_body(b"Forbidden".to_vec());

        Ok(())
    }
}
