use std::sync::Arc;

use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse, StatusCode,
    },
    Named,
};
use serde::{Deserialize, Serialize};

use crate::{
    core::context::SecurityContextHolder,
    webauthn::{RegistrationRequest, WebAuthnManager},
    web::{
        util::matcher::{PathPatternRequestMatcher, RequestMatcher},
        webauthn::{write_error, write_json},
    },
};

/// The answer of a registration: which user the credential was registered for,
/// and which credential it is.
#[derive(Serialize)]
struct RegistrationResponse {
    #[serde(rename = "userName")]
    user_name: String,
    #[serde(rename = "credentialId", with = "crate::webauthn::base64url")]
    credential_id: Vec<u8>,
}

/// The body that starts a registration.
#[derive(Debug, Default, Deserialize)]
struct RegistrationOptionsRequest {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

/// Serves the two endpoints of a registration ceremony.
///
/// A registration is for the user the request is authenticated as: the user
/// handle of the credential is that user, and the request never names one, so
/// the endpoint cannot be used to register a passkey for somebody else.
#[derive(Clone)]
pub struct WebAuthnRegistrationFilter {
    manager: Arc<WebAuthnManager>,
    options_matcher: Arc<dyn RequestMatcher>,
    ceremony_matcher: Arc<dyn RequestMatcher>,
}

impl WebAuthnRegistrationFilter {
    /// The endpoint the options of a registration are asked from.
    pub const DEFAULT_OPTIONS_PATTERN: &'static str = "/webauthn/register/options";

    /// The endpoint the answer of a registration is submitted to.
    pub const DEFAULT_PATTERN: &'static str = "/webauthn/register";

    /// Creates the endpoints of the given ceremonies.
    pub fn new(manager: Arc<WebAuthnManager>) -> Self {
        Self {
            manager,
            options_matcher: Arc::new(PathPatternRequestMatcher::path_pattern(
                None,
                Self::DEFAULT_OPTIONS_PATTERN,
            )),
            ceremony_matcher: Arc::new(PathPatternRequestMatcher::path_pattern(
                None,
                Self::DEFAULT_PATTERN,
            )),
        }
    }

    /// Answers the options of a registration.
    fn options(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        let Some(user_name) = authenticated_user_name() else {
            write_error(
                response,
                StatusCode::UNAUTHORIZED,
                "a registration runs for the authenticated user, and this request has none",
            );

            return Ok(());
        };

        let display_name = request
            .body_bytes()
            .filter(|body| !body.is_empty())
            .and_then(|body| serde_json::from_slice::<RegistrationOptionsRequest>(body).ok())
            .and_then(|request| request.display_name)
            .unwrap_or_else(|| user_name.clone());

        let options = self
            .manager
            .create_registration_options(&user_name, &display_name);
        write_json(response, StatusCode::OK, &options);

        Ok(())
    }

    /// Takes the answer to a registration, and stores the credential.
    fn ceremony(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        let Some(user_name) = authenticated_user_name() else {
            write_error(
                response,
                StatusCode::UNAUTHORIZED,
                "a registration runs for the authenticated user, and this request has none",
            );

            return Ok(());
        };

        let registration = match request
            .body_bytes()
            .ok_or_else(|| "the request carries no body".to_string())
            .and_then(|body| {
                RegistrationRequest::parse(body).map_err(|error| error.message().to_string())
            }) {
            Ok(registration) => registration,
            Err(message) => {
                write_error(response, StatusCode::BAD_REQUEST, &message);

                return Ok(());
            }
        };

        match self.manager.register(&user_name, &registration) {
            Ok(credential) => write_json(
                response,
                StatusCode::OK,
                &RegistrationResponse {
                    user_name: credential.user_name().to_string(),
                    credential_id: credential.credential_id().to_vec(),
                },
            ),
            Err(error) => write_error(response, StatusCode::BAD_REQUEST, error.message()),
        }

        Ok(())
    }
}

impl Named for WebAuthnRegistrationFilter {
    fn name(&self) -> &str {
        "WebAuthnRegistrationFilter"
    }
}

#[async_trait]
impl HttpFilter for WebAuthnRegistrationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.method() != next_web_core::http::HttpMethod::POST {
            return filter_chain.do_filter(request, response).await;
        }

        if self.options_matcher.matcher(request).is_match() {
            return self.options(request, response);
        }
        if self.ceremony_matcher.matcher(request).is_match() {
            return self.ceremony(request, response);
        }

        filter_chain.do_filter(request, response).await
    }
}

/// Returns the name of the user the request is authenticated as, if one is.
fn authenticated_user_name() -> Option<String> {
    let context = SecurityContextHolder::get_context();
    let authentication = context.get_authentication()?;
    if !authentication.is_authenticated() {
        return None;
    }

    authentication
        .principal()
        .map(|principal| principal.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use axum::{body::Body, extract::Request, response::Response};
    use next_web_core::{
        async_trait,
        http::{
            filter::{FilterError, HttpFilterChain},
            HttpRequest, HttpResponse, RequestBody,
        },
    };

    use super::*;
    use crate::{
        core::context::SecurityContextImpl,
        webauthn::{
            test_support::{
                attestation_object, authenticator_data, client_data_json, cose_key_of,
                registration_submission, signing_key, ATTESTED_CREDENTIAL_DATA, ORIGIN, RP_ID,
                USER_PRESENT,
            },
            InMemoryUserCredentialRepository, RelyingParty, UserCredentialRepository,
        },
        web::webauthn::WebAuthnAuthentication,
    };

    /// Returns a manager over a store the test can look into.
    fn manager() -> (Arc<WebAuthnManager>, Arc<InMemoryUserCredentialRepository>) {
        let credentials = Arc::new(InMemoryUserCredentialRepository::default());
        let mut manager = WebAuthnManager::new(RelyingParty::new(RP_ID, "The application"));
        manager.allowed_origins([ORIGIN]);
        manager.credential_repository(credentials.clone());

        (Arc::new(manager), credentials)
    }

    /// Signs the security context in as the given user, until the next test.
    fn signed_in_as(user_name: &str) {
        let authentication: Arc<dyn crate::core::Authentication> =
            Arc::new(WebAuthnAuthentication::new(user_name, Vec::new(), Vec::new()));
        SecurityContextHolder::set_context(Arc::new(SecurityContextImpl::new(authentication)));
    }

    /// Returns a `POST` with the given body.
    fn post(path: &str, body: &[u8]) -> Request {
        let mut request = Request::builder()
            .method("POST")
            .uri(format!("https://sp.example.org{path}"))
            .body(Body::empty())
            .expect("a valid request");
        request
            .extensions_mut()
            .insert(RequestBody::new(body.to_vec()));

        request
    }

    /// A chain that records whether it ran.
    #[derive(Clone, Default)]
    struct RecordingChain {
        ran: Arc<AtomicBool>,
    }

    impl RecordingChain {
        fn ran(&self) -> bool {
            self.ran.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl HttpFilterChain for RecordingChain {
        async fn do_filter(
            &self,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) -> Result<(), FilterError> {
            self.ran.store(true, Ordering::SeqCst);

            Ok(())
        }
    }

    /// Runs the filter against the request, and returns the response.
    async fn run(
        filter: &WebAuthnRegistrationFilter,
        request: &mut Request,
    ) -> (Response, bool) {
        let mut response = Response::default();
        let chain = RecordingChain::default();
        filter
            .do_filter(request, &mut response, &chain)
            .await
            .expect("the filter runs");

        (response, chain.ran())
    }

    /// Returns the JSON body of a response.
    async fn json(response: Response) -> serde_json::Value {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body is collected");
        serde_json::from_slice(&body).expect("the body is JSON")
    }

    #[tokio::test]
    async fn the_options_of_a_registration_are_answered_for_the_signed_in_user() {
        let (manager, _) = manager();
        let filter = WebAuthnRegistrationFilter::new(manager);
        signed_in_as("Max");
        let mut request = post(
            WebAuthnRegistrationFilter::DEFAULT_OPTIONS_PATTERN,
            br#"{"displayName":"Max Mustermann"}"#,
        );

        let (response, _) = run(&filter, &mut request).await;
        SecurityContextHolder::clear_context();
        let status = response.status();
        let body = json(response).await;

        assert_eq!(status, 200, "the options are answered");
        assert_eq!(body["rp"]["id"], RP_ID);
        assert_eq!(body["user"]["name"], "Max");
        assert_eq!(body["user"]["displayName"], "Max Mustermann");
    }

    #[tokio::test]
    async fn a_registration_without_a_signed_in_user_is_unauthorized() {
        let (manager, _) = manager();
        let filter = WebAuthnRegistrationFilter::new(manager);
        SecurityContextHolder::clear_context();
        let mut request = post(
            WebAuthnRegistrationFilter::DEFAULT_OPTIONS_PATTERN,
            b"",
        );

        let (response, _) = run(&filter, &mut request).await;

        assert_eq!(response.status(), 401);
    }

    #[tokio::test]
    async fn the_answer_of_a_registration_is_taken_and_the_credential_stored() {
        let (manager, credentials) = manager();
        let filter = WebAuthnRegistrationFilter::new(manager.clone());
        signed_in_as("Max");
        let options = manager.create_registration_options("Max", "Max");
        let cose_key = cose_key_of(&signing_key());
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(crate::webauthn::CREATE, options.challenge(), ORIGIN, false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let mut request = post(WebAuthnRegistrationFilter::DEFAULT_PATTERN, &submission);

        let (response, _) = run(&filter, &mut request).await;
        SecurityContextHolder::clear_context();
        let status = response.status();
        let body = json(response).await;

        assert_eq!(status, 200);
        assert_eq!(
            body["credentialId"],
            crate::webauthn::test_support::to_base64_url(b"the-credential")
        );
        assert!(
            credentials
                .find_by_credential_id(b"the-credential")
                .is_some(),
            "the credential is stored"
        );
    }

    #[tokio::test]
    async fn a_registration_that_does_not_hold_is_a_bad_request() {
        let (manager, credentials) = manager();
        let filter = WebAuthnRegistrationFilter::new(manager);
        signed_in_as("Max");
        let mut request = post(WebAuthnRegistrationFilter::DEFAULT_PATTERN, b"not a ceremony");

        let (response, _) = run(&filter, &mut request).await;
        SecurityContextHolder::clear_context();
        let status = response.status();
        let body = json(response).await;

        assert_eq!(status, 400);
        assert!(body["error"].is_string());
        assert!(
            credentials.find_by_credential_id(b"the-credential").is_none(),
            "nothing is stored"
        );
    }

    #[tokio::test]
    async fn a_request_of_another_method_is_passed_down_the_chain() {
        let (manager, _) = manager();
        let filter = WebAuthnRegistrationFilter::new(manager);
        let mut request = Request::builder()
            .method("GET")
            .uri("https://sp.example.org/webauthn/register/options")
            .body(Body::empty())
            .expect("a valid request");

        let (_, ran) = run(&filter, &mut request).await;

        assert!(ran, "a GET is not a ceremony");
    }
}
