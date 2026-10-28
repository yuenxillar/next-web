use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpMethod, HttpRequest, HttpResponse, StatusCode,
    },
    Named,
};
use serde::Deserialize;

use crate::{
    webauthn::WebAuthnManager,
    web::{
        authentication::{
            BaseAuthenticationProcessingFilter, BaseAuthenticationProcessingFilterExt,
        },
        util::matcher::{PathPatternRequestMatcher, RequestMatcher},
        webauthn::{write_error, write_json, WebAuthnAuthenticationConverter},
    },
};

/// The body that starts an authentication.
#[derive(Debug, Default, Deserialize)]
struct AuthenticationOptionsRequest {
    username: Option<String>,
}

/// Serves the two endpoints of an authentication ceremony.
///
/// The options endpoint hands the browser the challenge to sign. The other one
/// is a login: the assertion is converted into a token, the authentication
/// manager hands it to [`WebAuthnAuthenticationProvider`], and the success
/// handler finishes the login the way it does for a form.
///
/// [`WebAuthnAuthenticationProvider`]: crate::web::webauthn::WebAuthnAuthenticationProvider
#[derive(Clone)]
pub struct WebAuthnAuthenticationFilter {
    base: BaseAuthenticationProcessingFilter,
    manager: Arc<WebAuthnManager>,
    options_matcher: Arc<dyn RequestMatcher>,
}

impl WebAuthnAuthenticationFilter {
    /// The endpoint the options of an authentication are asked from.
    pub const DEFAULT_OPTIONS_PATTERN: &'static str = "/webauthn/authenticate/options";

    /// The endpoint the answer of an authentication is submitted to.
    pub const DEFAULT_PATTERN: &'static str = "/webauthn/authenticate";

    /// Creates the endpoints of the given ceremonies.
    pub fn new(manager: Arc<WebAuthnManager>) -> Self {
        let mut base = BaseAuthenticationProcessingFilter::new(Self::DEFAULT_PATTERN);
        base.set_authentication_converter(Arc::new(WebAuthnAuthenticationConverter));

        Self {
            base,
            manager,
            options_matcher: Arc::new(PathPatternRequestMatcher::path_pattern(
                None,
                Self::DEFAULT_OPTIONS_PATTERN,
            )),
        }
    }

    /// Answers the options of an authentication.
    ///
    /// The body names the user the ceremony is for, which restricts the
    /// credentials the browser offers to that user's. A body that names nobody
    /// is refused rather than answered with an unrestricted ceremony: the
    /// browser would then offer every passkey it has, including the credentials
    /// of other relying parties.
    fn options(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        let user_name = request
            .body_bytes()
            .filter(|body| !body.is_empty())
            .and_then(|body| serde_json::from_slice::<AuthenticationOptionsRequest>(body).ok())
            .and_then(|request| request.username);

        let Some(user_name) = user_name.filter(|name| !name.trim().is_empty()) else {
            write_error(
                response,
                StatusCode::BAD_REQUEST,
                "an authentication has to name the user it is for",
            );

            return Ok(());
        };

        write_json(
            response,
            StatusCode::OK,
            &self.manager.create_authentication_options(&user_name),
        );

        Ok(())
    }
}

impl Deref for WebAuthnAuthenticationFilter {
    type Target = BaseAuthenticationProcessingFilter;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for WebAuthnAuthenticationFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl BaseAuthenticationProcessingFilterExt for WebAuthnAuthenticationFilter {}

impl Named for WebAuthnAuthenticationFilter {
    fn name(&self) -> &str {
        "WebAuthnAuthenticationFilter"
    }
}

#[async_trait]
impl HttpFilter for WebAuthnAuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.method() == HttpMethod::POST
            && self.options_matcher.matcher(request).is_match()
        {
            return self.options(request, response);
        }

        BaseAuthenticationProcessingFilter::do_filter(request, response, filter_chain, self).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use axum::{body::Body, extract::Request, response::Response};
    use next_web_core::{
        async_trait,
        http::{
            filter::{FilterError, HttpFilterChain},
            HttpRequest, HttpResponse, RequestBody,
        },
    };

    use super::*;
    use crate::webauthn::{
        test_support::{register_credential, ORIGIN, RP_ID},
        InMemoryUserCredentialRepository, RelyingParty,
    };

    /// Returns a manager that registered `the-credential` for `Max`.
    fn manager() -> Arc<WebAuthnManager> {
        let mut manager = WebAuthnManager::new(RelyingParty::new(RP_ID, "The application"));
        manager.allowed_origins([ORIGIN]);
        manager.credential_repository(Arc::new(InMemoryUserCredentialRepository::default()));
        register_credential(&manager, b"the-credential", "Max");

        Arc::new(manager)
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

    /// Runs the filter against the request, and returns the response and whether
    /// the chain was continued.
    async fn run(
        filter: &WebAuthnAuthenticationFilter,
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
    async fn the_options_of_an_authentication_offer_the_credentials_of_the_user() {
        let filter = WebAuthnAuthenticationFilter::new(manager());
        let mut request = post(
            WebAuthnAuthenticationFilter::DEFAULT_OPTIONS_PATTERN,
            br#"{"username":"Max"}"#,
        );

        let (response, ran) = run(&filter, &mut request).await;
        let status = response.status();
        let body = json(response).await;

        assert!(!ran, "the options are answered here");
        assert_eq!(status, 200);
        assert_eq!(body["rpId"], RP_ID);
        assert_eq!(
            body["allowCredentials"][0]["id"],
            crate::webauthn::test_support::to_base64_url(b"the-credential")
        );
    }

    #[tokio::test]
    async fn an_authentication_that_names_no_user_is_a_bad_request() {
        let filter = WebAuthnAuthenticationFilter::new(manager());
        let mut request = post(WebAuthnAuthenticationFilter::DEFAULT_OPTIONS_PATTERN, b"{}");

        let (response, _) = run(&filter, &mut request).await;
        let status = response.status();
        let body = json(response).await;

        assert_eq!(status, 400);
        assert!(body["error"].is_string());
    }

    #[tokio::test]
    async fn a_request_that_is_not_a_ceremony_is_passed_down_the_chain() {
        let filter = WebAuthnAuthenticationFilter::new(manager());
        let mut request = Request::builder()
            .method("GET")
            .uri("https://sp.example.org/webauthn/authenticate/options")
            .body(Body::empty())
            .expect("a valid request");

        let (_, ran) = run(&filter, &mut request).await;

        assert!(ran, "a GET is not a ceremony");
    }
}
