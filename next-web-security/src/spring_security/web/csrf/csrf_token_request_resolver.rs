use tracing::trace;

use crate::web::csrf::CsrfToken;
use next_web_core::traits::http::http_request::HttpRequest;

/// Implementations of this interface are capable of resolving the token value of a CsrfToken from the
/// provided HttpRequest. Used by the CsrfFilter.
pub trait CsrfTokenRequestResolver {
    /// Returns the token value resolved from the provided HttpRequest and CsrfToken or Option::None if not available.
    fn resolve_csrf_token_value(
        &self,
        request: &mut dyn HttpRequest,
        csrf_token: &dyn CsrfToken,
    ) -> Option<String> {
        let mut actual_token = request.header(csrf_token.header_name());

        match actual_token {
            Some(token) => return Some(token.to_string()),
            None => trace!(
                "Did not find a CSRF token in the {} request header",
                csrf_token.header_name()
            ),
        };

        actual_token = request.parameter(csrf_token.parameter_name());

        match actual_token {
            Some(token) => return Some(token.to_string()),
            None => {
                trace!(
                    "Did not find a CSRF token in the {} request parameter",
                    csrf_token.parameter_name()
                );
                return None;
            }
        }
    }
}
