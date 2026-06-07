use next_web_core::traits::http::http_request::HttpRequest;
use tracing::trace;

use crate::web::csrf::CsrfToken;

pub trait CsrfTokenRequestResolver {
    fn resolve_csrf_token_value(
        &self,
        request: &mut dyn HttpRequest,
        csrf_token: &dyn CsrfToken,
    ) -> Option<String> {
        let mut actual_token = request.header(csrf_token.get_header_name());

        match actual_token {
            Some(token) => return Some(token.to_string()),
            None => trace!(
                "Did not find a CSRF token in the {} request header",
                csrf_token.get_header_name()
            ),
        };

        actual_token = request.parameter(csrf_token.get_parameter_name());

        match actual_token {
            Some(token) => return Some(token.to_string()),
            None => {
                trace!(
                    "Did not find a CSRF token in the {} request parameter",
                    csrf_token.get_parameter_name()
                );
                return None;
            }
        }
    }
}
