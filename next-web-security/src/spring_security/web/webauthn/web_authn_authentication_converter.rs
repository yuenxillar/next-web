use std::any::Any;

use next_web_core::http::HttpRequest;

use crate::{
    core::{Authentication, AuthenticationError, AuthenticationErrorKind},
    webauthn::AuthenticationRequest,
    web::{
        authentication::AuthenticationConverter, webauthn::WebAuthnAuthenticationToken,
    },
};

/// Converts the assertion a browser submitted into the token the provider
/// authenticates.
///
/// A submission that carries no body, or a body that is not the JSON of a
/// ceremony, is not an authentication attempt of this filter.
#[derive(Clone, Copy, Debug, Default)]
pub struct WebAuthnAuthenticationConverter;

impl AuthenticationConverter for WebAuthnAuthenticationConverter {
    fn convert(
        &self,
        request: &mut dyn HttpRequest,
    ) -> Result<Option<Box<dyn Authentication>>, AuthenticationError> {
        let Some(body) = request.body_bytes() else {
            return Ok(None);
        };
        if body.is_empty() {
            return Ok(None);
        }

        let request = AuthenticationRequest::parse(body).map_err(|error| {
            AuthenticationError::with_kind(error.message(), AuthenticationErrorKind::BadCredentials)
        })?;

        Ok(Some(Box::new(WebAuthnAuthenticationToken::new(request))))
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request};
    use next_web_core::http::RequestBody;

    use super::*;
    use crate::{
        webauthn::{
            test_support::{authenticate_submission, register_credential, ORIGIN, RP_ID},
            InMemoryUserCredentialRepository, RelyingParty, WebAuthnManager,
        },
        web::authentication::AuthenticationConverter,
    };

    /// Returns a `POST` to the authentication endpoint with the given body.
    fn post(body: &[u8]) -> Request {
        let mut request = Request::builder()
            .method("POST")
            .uri("https://sp.example.org/webauthn/authenticate")
            .body(Body::empty())
            .expect("a valid request");
        request
            .extensions_mut()
            .insert(RequestBody::new(body.to_vec()));

        request
    }

    /// Returns the submission of an assertion of a registered credential.
    fn submission() -> Vec<u8> {
        let mut manager = WebAuthnManager::new(RelyingParty::new(RP_ID, "The application"));
        manager.allowed_origins([ORIGIN]);
        manager.credential_repository(std::sync::Arc::new(
            InMemoryUserCredentialRepository::default(),
        ));
        register_credential(&manager, b"the-credential", "Max");

        authenticate_submission(&manager, b"the-credential", "Max", 1)
    }

    #[test]
    fn the_converter_reads_the_assertion_of_the_post() {
        let mut request = post(&submission());

        let authentication = WebAuthnAuthenticationConverter
            .convert(&mut request)
            .expect("the body is read")
            .expect("the body is an assertion");
        let token = (authentication.as_ref() as &dyn Any)
            .downcast_ref::<WebAuthnAuthenticationToken>()
            .expect("the authentication is a token");

        assert_eq!(token.request().credential_id(), b"the-credential");
    }

    #[test]
    fn a_post_without_a_body_is_not_an_authentication() {
        let mut request = post(b"");

        assert!(WebAuthnAuthenticationConverter
            .convert(&mut request)
            .expect("the body is read")
            .is_none());
    }

    #[test]
    fn a_body_that_is_not_a_ceremony_is_refused() {
        let mut request = post(b"not the JSON of a ceremony");

        let error = WebAuthnAuthenticationConverter
            .convert(&mut request)
            .expect_err("the body is not an assertion");

        assert!(
            error.to_string().contains("authentication request"),
            "the error names what it could not read, was: {error}"
        );
    }
}
