use std::any::Any;

use base64::Engine;
use next_web_core::http::HttpRequest;

use crate::{
    core::{Authentication, AuthenticationError, AuthenticationErrorKind},
    saml2::login::Saml2AuthenticationToken,
    web::authentication::AuthenticationConverter,
};

/// Converts a submitted assertion into the token the provider authenticates.
///
/// The asserting party posts a form to the assertion consumer endpoint: the
/// response, base64 encoded, and the state to return to. A submission that
/// carries no response is not an authentication attempt, and the endpoint
/// continues the chain.
#[derive(Clone, Copy, Debug, Default)]
pub struct Saml2AuthenticationConverter;

impl AuthenticationConverter for Saml2AuthenticationConverter {
    fn convert(
        &self,
        request: &mut dyn HttpRequest,
    ) -> Result<Option<Box<dyn Authentication>>, AuthenticationError> {
        let Some(body) = request.body_bytes() else {
            return Ok(None);
        };

        let mut response = None;
        let mut relay_state = None;
        for (name, value) in next_web_core::util::form_urlencoded::parse(body) {
            match name.as_ref() {
                "SAMLResponse" => response = Some(value.into_owned()),
                "RelayState" => relay_state = Some(value.into_owned()),
                _ => {}
            }
        }

        let Some(response) = response else {
            return Ok(None);
        };

        let decoded = base64::engine::general_purpose::STANDARD
            .decode(response.as_bytes())
            .map_err(|error| {
                AuthenticationError::with_kind(
                    format!("the SAML response is not base64: {error}"),
                    AuthenticationErrorKind::BadCredentials,
                )
            })?;
        let response_xml = String::from_utf8(decoded).map_err(|error| {
            AuthenticationError::with_kind(
                format!("the SAML response is not UTF-8: {error}"),
                AuthenticationErrorKind::BadCredentials,
            )
        })?;

        Ok(Some(Box::new(
            Saml2AuthenticationToken::new(
                registration_id(request).unwrap_or_default(),
                response_xml,
                relay_state.as_deref(),
            )
            .with_base_url(crate::web::saml2::request_base_url(request)),
        )))
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Returns the registration a request was submitted to.
///
/// The assertion consumer endpoint ends with the registration id, which is what
/// the asserting party was told to post to.
fn registration_id(request: &dyn HttpRequest) -> Option<String> {
    request
        .path()
        .split('/')
        .filter(|segment| !segment.is_empty())
        .next_back()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request};
    use base64::Engine;
    use next_web_core::http::RequestBody;

    use super::*;
    use crate::saml2::login::Saml2AuthenticationToken;
    use crate::web::authentication::AuthenticationConverter;

    /// Returns a `POST` of a url-encoded form to the assertion consumer service.
    fn form_post(registration_id: &str, body: &str) -> Request {
        let mut request = Request::builder()
            .method("POST")
            .uri(format!("https://sp.example.org/login/saml2/sso/{registration_id}"))
            .body(Body::empty())
            .expect("a valid request");
        request
            .extensions_mut()
            .insert(RequestBody::new(body.as_bytes().to_vec()));

        request
    }

    /// Returns the body of a form whose response is the given document.
    fn form_body(document: &str, relay_state: Option<&str>) -> String {
        let encoded =
            base64::engine::general_purpose::STANDARD.encode(document.as_bytes());
        let mut body = format!("SAMLResponse={}", urlencoding::encode(&encoded));
        if let Some(relay_state) = relay_state {
            body.push_str("&RelayState=");
            body.push_str(&urlencoding::encode(relay_state));
        }

        body
    }

    #[test]
    fn the_converter_reads_the_response_of_the_post() {
        let mut request = form_post("okta", &form_body("<saml2p:Response/>", Some("/start")));

        let authentication = Saml2AuthenticationConverter
            .convert(&mut request)
            .expect("the body is read")
            .expect("the form carries a response");
        let token = (authentication.as_ref() as &dyn std::any::Any)
            .downcast_ref::<Saml2AuthenticationToken>()
            .expect("the authentication is a token");

        assert_eq!(token.registration_id(), "okta");
        assert_eq!(token.response_xml(), "<saml2p:Response/>");
        assert_eq!(token.relay_state(), Some("/start"));
    }

    #[test]
    fn a_form_without_a_response_is_not_an_authentication() {
        let mut request = form_post("okta", "RelayState=%2Fstart");

        assert!(Saml2AuthenticationConverter
            .convert(&mut request)
            .expect("the body is read")
            .is_none());
    }

    #[test]
    fn a_response_that_is_not_base64_is_rejected() {
        let mut request = form_post("okta", "SAMLResponse=not-base64%21");

        let error = Saml2AuthenticationConverter
            .convert(&mut request)
            .expect_err("the response cannot be decoded");

        assert!(
            error.to_string().contains("SAML"),
            "the error names the response, was: {error}"
        );
    }
}
