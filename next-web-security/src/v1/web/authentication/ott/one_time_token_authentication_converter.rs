use std::any::Any;

use next_web_core::{http::HttpRequest, util::StringUtils};
use tracing::debug;

use crate::{
    authentication::ott::one_time_token_authentication_token::OneTimeTokenAuthenticationToken,
    core::{Authentication, AuthenticationError},
    web::authentication::authentication_converter::AuthenticationConverter,
};

#[derive(Default, Clone)]
pub struct OneTimeTokenAuthenticationConverter;

impl AuthenticationConverter for OneTimeTokenAuthenticationConverter {
    fn convert(
        &self,
        request: &mut dyn HttpRequest,
    ) -> Result<Option<Box<dyn Authentication>>, AuthenticationError> {
        // The submit page posts the token in a form, so it is read from the body
        // as well as from the query.
        let token = request
            .form_parameter("token")
            .map(|value| value.into_owned())
            .unwrap_or_default();

        if !StringUtils::has_text(token.as_str()) {
            debug!("No token found in request");
            return Ok(None);
        }
        Ok(Some(Box::new(
            OneTimeTokenAuthenticationToken::unauthenticated(token.as_str()),
        )))
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use axum::{body::Body, extract::Request};
    use next_web_core::http::RequestBody;

    use super::*;

    /// Returns a `POST` of the submit form.
    fn submit_form(body: &str) -> Request {
        let mut request = Request::builder()
            .method("POST")
            .uri("https://sp.example.org/login/ott")
            .body(Body::empty())
            .expect("a valid request");
        request
            .extensions_mut()
            .insert(RequestBody::new(body.as_bytes().to_vec()));

        request
    }

    #[test]
    fn the_token_is_read_from_the_form_body() {
        let mut request = submit_form("token=the-token");

        let authentication = OneTimeTokenAuthenticationConverter
            .convert(&mut request)
            .expect("the request converts")
            .expect("the token of the form is found");

        assert_eq!(
            authentication.of(),
            TypeId::of::<OneTimeTokenAuthenticationToken>()
        );
    }

    #[test]
    fn a_form_without_a_token_converts_to_nothing() {
        let mut request = submit_form("other=value");

        assert!(
            OneTimeTokenAuthenticationConverter
                .convert(&mut request)
                .expect("the request converts")
                .is_none(),
            "a form that carries no token is not an authentication attempt"
        );
    }
}
