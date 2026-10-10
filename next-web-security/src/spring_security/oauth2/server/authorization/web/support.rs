//! The helpers the endpoints of the authorization server share.

use next_web_core::{
    error::BoxError,
    filter::FilterError,
    http::StatusCode,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use serde_json::Value;

use crate::{
    core::{AuthenticationError, AuthenticationErrorKind},
    oauth2::server::authorization::error::OAuth2Error,
    web::{DefaultRedirectStrategy, RedirectStrategy},
};

/// Writes a JSON body to the response, which commits the response.
///
/// # Arguments
///
/// * `response` - The response of the endpoint.
/// * `status` - The status of the response.
/// * `body` - The JSON body.
pub(crate) fn json_response(response: &mut dyn HttpResponse, status: StatusCode, body: &Value) {
    response.insert_header("Content-Type", "application/json;charset=UTF-8");
    response.insert_header("Cache-Control", "no-store");
    response.insert_header("Pragma", "no-cache");
    response.set_status_code(status);
    response.set_body(serde_json::to_vec(body).unwrap_or_default());
}

/// Writes the error response of an endpoint.
///
/// A client authentication failure is reported with `401` and the
/// `WWW-Authenticate` header of the specification, which is what a client that
/// has to retry with credentials expects.
///
/// # Arguments
///
/// * `response` - The response of the endpoint.
/// * `error` - The error of the endpoint.
pub(crate) fn error_response(response: &mut dyn HttpResponse, error: &OAuth2Error) {
    let status = if error.error() == "invalid_client" {
        response.insert_header("WWW-Authenticate", &error.www_authenticate());
        StatusCode::UNAUTHORIZED
    } else {
        StatusCode::BAD_REQUEST
    };

    json_response(response, status, &error.to_json());
}

/// Returns the error that reports that the resource owner has to log in first.
///
/// The error is translated by the error translation filter, which is what sends
/// the resource owner to the login page and lets the request continue after the
/// login.
pub(crate) fn resource_owner_required() -> FilterError {
    FilterError::Chain(next_web_core::filter::FilterChainError::AnyError(Box::new(
        AuthenticationError::with_kind(
            "Full authentication is required to access the authorization endpoint",
            AuthenticationErrorKind::InsufficientAuthentication,
        ),
    )))
}

/// Builds a redirect URI with the given parameters.
///
/// # Arguments
///
/// * `base_uri` - The redirect URI of the client.
/// * `parameters` - The parameters of the redirect.
pub(crate) fn redirect_url(base_uri: &str, parameters: &[(&str, &str)]) -> String {
    if parameters.is_empty() {
        return base_uri.to_string();
    }

    let separator = if base_uri.contains('?') { '&' } else { '?' };
    let query = parameters
        .iter()
        .map(|(name, value)| format!("{}={}", name, urlencoding::encode(value)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{base_uri}{separator}{query}")
}

/// Sends the client back to the redirect URI of its authorization request.
///
/// # Arguments
///
/// * `request` - The request of the endpoint.
/// * `response` - The response of the endpoint.
/// * `redirect_uri` - The redirect URI of the client.
/// * `parameters` - The parameters of the redirect.
pub(crate) fn send_redirect(
    request: &dyn HttpRequest,
    response: &mut dyn HttpResponse,
    redirect_uri: &str,
    parameters: &[(&str, &str)],
) -> Result<(), FilterError> {
    let strategy = DefaultRedirectStrategy::default();
    strategy
        .send_redirect(request, response, &redirect_url(redirect_uri, parameters))
        .map_err(|error: BoxError| {
            FilterError::Chain(next_web_core::filter::FilterChainError::Boxed(error))
        })
}

/// Verifies the proof key of an authorization code exchange.
///
/// # Arguments
///
/// * `code_challenge` - The challenge of the authorization request.
/// * `code_challenge_method` - The method of the challenge.
/// * `code_verifier` - The verifier of the token request.
pub(crate) fn verify_proof_key(
    code_challenge: &str,
    code_challenge_method: &str,
    code_verifier: &str,
) -> bool {
    match code_challenge_method {
        "S256" => {
            use sha2::{Digest, Sha256};

            let digest = Sha256::digest(code_verifier.as_bytes());
            super::super::jose::base64_url(&digest) == code_challenge
        }
        _ => code_challenge == code_verifier,
    }
}

#[cfg(test)]
mod tests {
    use super::{redirect_url, verify_proof_key};

    #[test]
    fn a_redirect_adds_its_parameters_to_the_uri() {
        assert_eq!(
            redirect_url(
                "https://client.example/callback",
                &[("code", "a b"), ("state", "s")]
            ),
            "https://client.example/callback?code=a%20b&state=s"
        );
        assert_eq!(
            redirect_url(
                "https://client.example/callback?existing=1",
                &[("code", "c")]
            ),
            "https://client.example/callback?existing=1&code=c"
        );
        assert_eq!(
            redirect_url("https://client.example/callback", &[]),
            "https://client.example/callback"
        );
    }

    #[test]
    fn a_plain_proof_key_matches_the_verifier() {
        assert!(verify_proof_key("verifier", "plain", "verifier"));
        assert!(!verify_proof_key("verifier", "plain", "other"));
    }

    #[test]
    fn an_s256_proof_key_is_the_digest_of_the_verifier() {
        // The digest of "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

        assert!(verify_proof_key(challenge, "S256", verifier));
        assert!(!verify_proof_key(challenge, "S256", "other"));
    }
}
