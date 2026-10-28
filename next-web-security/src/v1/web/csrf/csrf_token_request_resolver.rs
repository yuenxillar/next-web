use tracing::trace;

use crate::web::csrf::CsrfToken;
use next_web_core::http::HttpRequest;

/// Implementations of this interface are capable of resolving the token value of a CsrfToken from the
/// provided HttpRequest. Used by the CsrfFilter.
pub trait CsrfTokenRequestResolver {
    /// Returns the token value resolved from the provided HttpRequest and CsrfToken or Option::None if not available.
    fn resolve_csrf_token_value(
        &self,
        request: &mut dyn HttpRequest,
        csrf_token: &dyn CsrfToken,
    ) -> Option<String> {
        let actual_token = request.header(csrf_token.header_name());

        match actual_token {
            Some(token) => return Some(token.to_string()),
            None => trace!(
                "Did not find a CSRF token in the {} request header",
                csrf_token.header_name()
            ),
        };

        // A form carries the token in its body, not in the query, so the
        // parameter is resolved the way a servlet request resolves one.
        match request.form_parameter(csrf_token.parameter_name()) {
            Some(token) => Some(token.into_owned()),
            None => {
                trace!(
                    "Did not find a CSRF token in the {} request parameter",
                    csrf_token.parameter_name()
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request};
    use next_web_core::http::RequestBody;

    use super::*;
    use crate::web::csrf::DefaultCsrfToken;

    /// A resolver that keeps the default resolution of the trait.
    struct Resolver;

    impl CsrfTokenRequestResolver for Resolver {}

    /// Returns a `POST` of a form.
    fn form_post(body: &str) -> Request {
        let mut request = Request::builder()
            .method("POST")
            .uri("https://sp.example.org/do")
            .body(Body::empty())
            .expect("a valid request");
        request
            .extensions_mut()
            .insert(RequestBody::new(body.as_bytes().to_vec()));

        request
    }

    #[test]
    fn the_token_is_resolved_from_a_form_body() {
        let token = DefaultCsrfToken::new("X-CSRF-TOKEN", "_csrf", "the-token");
        let mut request = form_post("_csrf=the-token");

        let resolved = Resolver.resolve_csrf_token_value(&mut request, &token);

        assert_eq!(
            resolved.as_deref(),
            Some("the-token"),
            "the token of the form body protects the submission"
        );
    }

    #[test]
    fn a_submission_without_the_token_resolves_to_nothing() {
        let token = DefaultCsrfToken::new("X-CSRF-TOKEN", "_csrf", "the-token");
        let mut request = form_post("other=value");

        assert_eq!(
            Resolver.resolve_csrf_token_value(&mut request, &token),
            None
        );
    }
}
