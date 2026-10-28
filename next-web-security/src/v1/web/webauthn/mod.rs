//! The endpoints that run a Web Authentication ceremony.
//!
//! Four endpoints, named after the ones the reference implementation serves:
//! `/webauthn/register/options`, `/webauthn/register`,
//! `/webauthn/authenticate/options`, and `/webauthn/authenticate`. The first of
//! each pair hands the browser the options of a ceremony; the second takes the
//! answer to it.
//!
//! A registration runs for the user the request is authenticated as, so the
//! endpoints cannot be used to register a credential for somebody else. An
//! authentication is a login: the assertion is verified by
//! [`WebAuthnManager`](crate::webauthn::WebAuthnManager), the user the credential
//! belongs to is loaded, and the chain continues as it does after any other
//! login.

mod web_authn_authentication;
mod web_authn_authentication_converter;
mod web_authn_authentication_filter;
mod web_authn_authentication_provider;
mod web_authn_authentication_token;
mod web_authn_registration_filter;

use next_web_core::http::{HttpResponse, StatusCode};
use serde::Serialize;

pub use web_authn_authentication::WebAuthnAuthentication;
pub use web_authn_authentication_converter::WebAuthnAuthenticationConverter;
pub use web_authn_authentication_filter::WebAuthnAuthenticationFilter;
pub use web_authn_authentication_provider::WebAuthnAuthenticationProvider;
pub use web_authn_authentication_token::WebAuthnAuthenticationToken;
pub use web_authn_registration_filter::WebAuthnRegistrationFilter;

/// Writes a value as the JSON body of a response.
pub(crate) fn write_json(
    response: &mut dyn HttpResponse,
    status: StatusCode,
    value: &impl Serialize,
) {
    match serde_json::to_vec(value) {
        Ok(json) => {
            response.set_status_code(status);
            response.insert_header("Content-Type", "application/json");
            response.set_body(json);
        }
        Err(error) => {
            tracing::error!("the response of a webauthn endpoint could not be written: {error}");
            write_error(
                response,
                StatusCode::INTERNAL_SERVER_ERROR,
                "the response could not be written",
            );
        }
    }
}

/// Writes a refusal, and the reason of it, as the JSON body of a response.
pub(crate) fn write_error(response: &mut dyn HttpResponse, status: StatusCode, message: &str) {
    let body = serde_json::json!({ "error": message });

    response.set_status_code(status);
    response.insert_header("Content-Type", "application/json");
    response.set_body(serde_json::to_vec(&body).unwrap_or_default());
}
