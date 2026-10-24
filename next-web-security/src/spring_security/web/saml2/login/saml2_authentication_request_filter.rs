use std::sync::Arc;

use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse, StatusCode,
    },
    Named,
};

use crate::{
    saml2::{
        login::{Saml2AuthenticationRequest, Saml2AuthenticationRequestRepository},
        registration::RelyingPartyRegistrationRepository,
    },
    web::util::matcher::{PathPatternRequestMatcher, RequestMatcher},
};

/// How long a started login may be answered.
const REQUEST_LIFETIME_MINUTES: i64 = 5;

/// Serves the endpoint that starts a login at the asserting party.
///
/// A request to the endpoint of a registration is answered with a redirect to
/// the single sign-on service of the registration, carrying the authentication
/// request the asserting party needs to identify the service provider.
#[derive(Clone)]
pub struct Saml2AuthenticationRequestFilter {
    registrations: Arc<dyn RelyingPartyRegistrationRepository>,
    requests: Arc<dyn Saml2AuthenticationRequestRepository>,
    request_matcher: Arc<dyn RequestMatcher>,
}

impl Saml2AuthenticationRequestFilter {
    /// The endpoint a login is started on, by default.
    pub const DEFAULT_FILTER_PROCESSES_URI: &'static str = "/saml2/authenticate/{registrationId}";

    /// Creates a filter over the given registrations and request repository.
    pub fn new(
        registrations: Arc<dyn RelyingPartyRegistrationRepository>,
        requests: Arc<dyn Saml2AuthenticationRequestRepository>,
    ) -> Self {
        Self {
            registrations,
            requests,
            request_matcher: Arc::new(PathPatternRequestMatcher::path_pattern(
                None,
                Self::DEFAULT_FILTER_PROCESSES_URI,
            )),
        }
    }

    /// Uses the given matcher to decide which requests start a login.
    pub fn set_request_matcher(&mut self, request_matcher: Arc<dyn RequestMatcher>) {
        self.request_matcher = request_matcher;
    }
}

impl Named for Saml2AuthenticationRequestFilter {
    fn name(&self) -> &str {
        "Saml2AuthenticationRequestFilter"
    }
}

#[async_trait]
impl HttpFilter for Saml2AuthenticationRequestFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let matcher = self.request_matcher.matcher(request);
        if !matcher.is_match() {
            return filter_chain.do_filter(request, response).await;
        }

        let registration_id = matcher
            .get_variables()
            .and_then(|variables| variables.get("registrationId"));
        let Some(registration) = registration_id
            .and_then(|registration_id| self.registrations.find_by_registration_id(registration_id))
        else {
            response.set_status_code(StatusCode::UNAUTHORIZED);
            return Ok(());
        };

        let base_url = crate::web::saml2::request_base_url(request);
        let request_url = match Saml2AuthenticationRequest::create(&registration, &base_url) {
            Ok(authentication_request) => {
                // The response is only believed when it answers a login this
                // application started, so the request is remembered until it
                // answers or expires.
                self.requests.save(
                    registration.registration_id(),
                    authentication_request.id(),
                    chrono::Utc::now()
                        + chrono::Duration::minutes(REQUEST_LIFETIME_MINUTES),
                );

                authentication_request.to_redirect_url(None)
            }
            Err(error) => {
                // A registration that cannot start a login is a configuration
                // error; the caller sees the same answer as an unknown one.
                tracing::warn!("a SAML 2.0 login cannot be started: {error}");
                Err(error)
            }
        };

        match request_url {
            Ok(request_url) => {
                response.set_redirect(&request_url);
                Ok(())
            }
            Err(_) => {
                response.set_status_code(StatusCode::UNAUTHORIZED);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use axum::{body::Body, extract::Request, response::Response};
    use next_web_core::{
        async_trait,
        http::{
            filter::{FilterError, HttpFilterChain},
            HttpRequest, HttpResponse,
        },
    };

    use super::*;
    use crate::saml2::registration::{
        InMemoryRelyingPartyRegistrationRepository, RelyingPartyRegistration,
    };

    /// Returns a registration configured for an identity provider.
    fn registration() -> RelyingPartyRegistration {
        RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("sp-entity-id")
            .assertion_consumer_service_location("https://sp.example.org/login/saml2/sso/okta")
            .asserting_party_entity_id("idp-entity-id")
            .single_sign_on_service_location("https://idp.example.org/sso")
            .build()
    }

    /// Records the requests that were saved.
    #[derive(Default)]
    struct RecordingRepository {
        saved: std::sync::RwLock<Vec<(String, String)>>,
    }

    impl crate::saml2::login::Saml2AuthenticationRequestRepository for RecordingRepository {
        fn save(&self, registration_id: &str, request_id: &str, _expires_at: chrono::DateTime<chrono::Utc>) {
            self.saved
                .write()
                .expect("the recording repository is not poisoned")
                .push((registration_id.to_owned(), request_id.to_owned()));
        }

        fn consume(&self, _request_id: &str) -> Option<String> {
            None
        }
    }

    /// Returns a filter over one registration and the given request repository.
    fn filter_with(
        requests: Arc<dyn crate::saml2::login::Saml2AuthenticationRequestRepository>,
    ) -> Saml2AuthenticationRequestFilter {
        Saml2AuthenticationRequestFilter::new(
            Arc::new(InMemoryRelyingPartyRegistrationRepository::new(vec![
                registration(),
            ])),
            requests,
        )
    }

    /// Returns a filter over one registration.
    fn filter() -> Saml2AuthenticationRequestFilter {
        filter_with(Arc::new(RecordingRepository::default()))
    }

    /// Returns a `GET` request for an absolute URL.
    fn request(url: &str) -> Request {
        Request::builder()
            .method("GET")
            .uri(url)
            .body(Body::empty())
            .expect("a valid request")
    }

    /// Records whether the rest of the chain ran.
    #[derive(Clone, Default)]
    struct RecordingChain {
        ran: Arc<AtomicBool>,
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

    /// Runs the filter and returns the response and whether the chain ran.
    async fn run(url: &str) -> (Response, bool) {
        let mut request = request(url);
        let mut response = Response::default();
        let chain = RecordingChain::default();

        filter()
            .do_filter(&mut request, &mut response, &chain)
            .await
            .expect("the filter runs");

        (response, chain.ran.load(Ordering::SeqCst))
    }

    #[tokio::test]
    async fn the_login_redirects_to_the_asserting_party() {
        let (response, chain_ran) =
            run("https://sp.example.org/saml2/authenticate/okta").await;

        assert!(!chain_ran, "the request is answered by the filter");
        assert!(response.status().is_redirection(), "status was {}", response.status());
        let location = response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .expect("the redirect has a location");
        assert!(
            location.starts_with("https://idp.example.org/sso?SAMLRequest="),
            "location was: {location}"
        );
    }

    #[tokio::test]
    async fn an_unknown_registration_is_unauthorized() {
        let (response, chain_ran) =
            run("https://sp.example.org/saml2/authenticate/azure").await;

        assert!(!chain_ran);
        assert_eq!(response.status(), 401);
    }

    #[tokio::test]
    async fn a_request_that_does_not_match_is_passed_down_the_chain() {
        let (response, chain_ran) = run("https://sp.example.org/login/okta").await;

        assert!(chain_ran, "the rest of the chain handles the request");
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn the_started_login_is_remembered() {
        let requests = Arc::new(RecordingRepository::default());
        let mut request = request("https://sp.example.org/saml2/authenticate/okta");
        let mut response = Response::default();

        filter_with(requests.clone())
            .do_filter(&mut request, &mut response, &RecordingChain::default())
            .await
            .expect("the filter runs");

        let saved = requests
            .saved
            .read()
            .expect("the recording repository is not poisoned");
        assert_eq!(saved.len(), 1, "one request was saved");
        assert_eq!(saved[0].0, "okta");
        assert!(saved[0].1.starts_with('_'), "the saved id was: {}", saved[0].1);
    }
}
