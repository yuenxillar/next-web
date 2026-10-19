use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

use next_web_context::ApplicationContextExt;
use next_web_core::{ApplicationContext, Required};

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    core::userdetails::UserDetailsService,
    saml2::{
        login::{
            AssertionReplayCache, AssertionVerifier, InMemoryAssertionReplayCache,
            InMemorySaml2AuthenticationRequestRepository, RejectingAssertionVerifier,
            Saml2AuthenticationProvider, Saml2AuthenticationRequestRepository, SignatureVerifier,
            StrictAssertionVerifier,
        },
        registration::RelyingPartyRegistrationRepository,
    },
    web::{
        authentication::{BasicAuthenticationFilter, SimpleUrlAuthenticationFailureHandler},
        default_security_filter_chain::DefaultSecurityFilterChain,
        saml2::login::{Saml2AuthenticationFilter, Saml2AuthenticationRequestFilter},
    },
};

/// Configures SAML 2.0 login.
///
/// The configurer registers the two endpoints a login needs — the one a login is
/// started on, and the assertion consumer one the asserting party posts to — and
/// the provider that authenticates the posted response. A response is only
/// believed when an [`AssertionVerifier`] says so; without one the shipped
/// [`RejectingAssertionVerifier`] refuses every response.
#[derive(Clone)]
pub struct Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    registrations: Option<Arc<dyn RelyingPartyRegistrationRepository>>,
    user_details_service: Option<Arc<dyn UserDetailsService>>,
    assertion_verifier: Option<Arc<dyn AssertionVerifier>>,
    signature_verifier: Option<Arc<dyn SignatureVerifier>>,
    requests: Option<Arc<dyn Saml2AuthenticationRequestRepository>>,
    replay: Option<Arc<dyn AssertionReplayCache>>,
    clock_skew_seconds: i64,
    context: Arc<RwLock<Box<dyn ApplicationContext>>>,

    base: BaseHttpConfigurer<Self, H>,
}

impl<H> Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Creates a configurer that resolves what it needs from the given context.
    pub fn new(context: Arc<RwLock<Box<dyn ApplicationContext>>>) -> Self {
        Self {
            context,
            ..Self::default()
        }
    }

    /// Sets the relying party registrations a login is performed with.
    pub fn relying_party_registration_repository(
        &mut self,
        registrations: Arc<dyn RelyingPartyRegistrationRepository>,
    ) -> &mut Self {
        self.registrations = Some(registrations);
        self
    }

    /// Sets the user details service a name id is loaded with.
    pub fn user_details_service(
        &mut self,
        user_details_service: Arc<dyn UserDetailsService>,
    ) -> &mut Self {
        self.user_details_service = Some(user_details_service);
        self
    }

    /// Sets the verifier that decides whether a response may be believed.
    ///
    /// Without one, every response is refused: a response whose signature was
    /// not checked proves nothing.
    pub fn assertion_verifier(
        &mut self,
        assertion_verifier: Arc<dyn AssertionVerifier>,
    ) -> &mut Self {
        self.assertion_verifier = Some(assertion_verifier);
        self
    }

    /// Sets the implementation that verifies the signature of a response.
    ///
    /// With one set, the checks of the profile — the two identities, the
    /// endpoint, the window, the request it answers, and re-use — are performed
    /// by [`StrictAssertionVerifier`] around it.
    pub fn signature_verifier(
        &mut self,
        signature_verifier: Arc<dyn SignatureVerifier>,
    ) -> &mut Self {
        self.signature_verifier = Some(signature_verifier);
        self
    }

    /// Sets where the started logins are remembered.
    pub fn authentication_request_repository(
        &mut self,
        requests: Arc<dyn Saml2AuthenticationRequestRepository>,
    ) -> &mut Self {
        self.requests = Some(requests);
        self
    }

    /// Sets where the used assertions are remembered.
    pub fn assertion_replay_cache(&mut self, replay: Arc<dyn AssertionReplayCache>) -> &mut Self {
        self.replay = Some(replay);
        self
    }

    /// Tolerates the given number of seconds of clock difference.
    pub fn clock_skew(&mut self, seconds: i64) -> &mut Self {
        self.clock_skew_seconds = seconds;
        self
    }

    /// Returns the verifier a response is judged by.
    fn build_verifier(&self) -> Arc<dyn AssertionVerifier> {
        if let Some(verifier) = self.assertion_verifier.clone() {
            return verifier;
        }

        let Some(signature) = self.signature_verifier.clone() else {
            // Silence here would mean a login endpoint that starts logins it
            // can never finish, with nothing to explain why.
            tracing::warn!(
                "saml2Login() is configured without a SignatureVerifier, so every assertion is \
                 refused and no login can complete; supply one that verifies the signature of a \
                 response"
            );
            return Arc::new(RejectingAssertionVerifier);
        };

        Arc::new(
            StrictAssertionVerifier::new(
                signature,
                self.request_repository(),
                self.replay_cache(),
            )
            .clock_skew(self.clock_skew_seconds),
        )
    }

    /// Returns where the started logins are remembered.
    fn request_repository(&self) -> Arc<dyn Saml2AuthenticationRequestRepository> {
        self.requests
            .clone()
            .unwrap_or_else(|| Arc::new(InMemorySaml2AuthenticationRequestRepository::default()))
    }

    /// Returns where the used assertions are remembered.
    fn replay_cache(&self) -> Arc<dyn AssertionReplayCache> {
        self.replay
            .clone()
            .unwrap_or_else(|| Arc::new(InMemoryAssertionReplayCache::default()))
    }

    /// Resolves the registrations a login is performed with.
    fn resolve_registrations(&self, http: &H) -> Option<Arc<dyn RelyingPartyRegistrationRepository>> {
        if let Some(registrations) = &self.registrations {
            return Some(registrations.clone());
        }

        if let Some(registrations) =
            http.shared_object::<Arc<dyn RelyingPartyRegistrationRepository>>()
        {
            return Some(registrations.clone());
        }

        let context = self
            .context
            .write()
            .expect("the application context lock is poisoned");

        (*context)
            .get_singletons_by_type::<Arc<dyn RelyingPartyRegistrationRepository>>()
            .first()
            .map(|registrations| (*registrations).clone())
    }

    /// Resolves the user details service a name id is loaded with.
    fn resolve_user_details_service(&self) -> Option<Arc<dyn UserDetailsService>> {
        if let Some(user_details_service) = &self.user_details_service {
            return Some(user_details_service.clone());
        }

        let context = self
            .context
            .write()
            .expect("the application context lock is poisoned");

        (*context)
            .get_singletons_by_type::<Arc<dyn UserDetailsService>>()
            .first()
            .map(|user_details_service| (*user_details_service).clone())
    }
}

impl<H> Default for Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            registrations: None,
            user_details_service: None,
            assertion_verifier: None,
            signature_verifier: None,
            requests: None,
            replay: None,
            clock_skew_seconds: 0,
            context: Arc::new(RwLock::new(
                Box::new(super::super::builders::StandaloneApplicationContext::default())
                    as Box<dyn ApplicationContext>,
            )),
            base: Default::default(),
        }
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn init(&mut self, http: &mut H) {
        let Some(registrations) = self.resolve_registrations(http) else {
            panic!("relyingPartyRegistrationRepository cannot be null");
        };
        let Some(user_details_service) = self.resolve_user_details_service() else {
            panic!(
                "A UserDetailsService is required to enable saml2Login(). \
                 Please provide it as a bean or pass it to the saml2Login() DSL."
            );
        };

        self.registrations = Some(registrations.clone());
        self.user_details_service = Some(user_details_service.clone());

        // One repository serves both ends: the endpoint remembers the login it
        // started, and the verifier consumes the response that answers it.
        if self.requests.is_none() {
            self.requests = Some(Arc::new(
                InMemorySaml2AuthenticationRequestRepository::default(),
            ));
        }

        let verifier = self.build_verifier();

        http.authentication_provider(Arc::new(Saml2AuthenticationProvider::new(
            registrations,
            user_details_service,
            verifier,
        )));
    }

    fn configure(&mut self, http: &mut H) {
        let Some(registrations) = self.registrations.clone() else {
            // `init` is where a chain that forgot the registration repository is
            // reported, so a configurer that was not initialized has nothing to
            // register.
            return;
        };

        // Both endpoints sit ahead of the basic authentication filter, the point
        // of the chain the other form-based logins are processed at.
        http.add_filter_before::<Saml2AuthenticationRequestFilter, BasicAuthenticationFilter>(
            Saml2AuthenticationRequestFilter::new(registrations, self.request_repository()),
        );

        let mut filter = Saml2AuthenticationFilter::default();
        if let Some(authentication_manager) =
            http.shared_object::<Arc<dyn crate::authorization::AuthenticationManager>>()
        {
            filter.set_authentication_manager(authentication_manager.clone());
        }
        // A login has to outlive the request that made it: the context is saved
        // to the repository the security context configurer built, which is the
        // request attribute and the session.
        if let Some(security_context_configurer) =
            http.configurer::<crate::config::web::configurers::SecurityContextConfigurer<H>>()
        {
            let repository = security_context_configurer.get_security_context_repository(http);
            filter.set_security_context_repository(repository);
        }
        // A refused login sends the user back to the login page, the way the
        // reference implementation answers it.
        filter.set_failure_handler(Arc::new(SimpleUrlAuthenticationFailureHandler::new(
            "/login?error",
        )));

        http.add_filter_before::<Saml2AuthenticationFilter, BasicAuthenticationFilter>(filter);
    }
}

impl<H> Deref for Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for Saml2LoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    use next_web_core::ApplicationContext;

    use super::*;
    use crate::{
        config::{
            authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
            web::builders::{HttpSecurity, StandaloneApplicationContext},
        },
        core::userdetails::{User, UserDetailsService},
        provisioning::InMemoryUserDetailsManager,
        saml2::login::{AssertionVerifier, RejectingAssertionVerifier, Saml2Response},
        saml2::registration::{
            InMemoryRelyingPartyRegistrationRepository, RelyingPartyRegistration,
        },
        saml2::Saml2Error,
        web::security_filter_chain::SecurityFilterChain,
    };

    /// Returns a context that contributes no relying party registrations.
    fn context() -> Arc<RwLock<Box<dyn ApplicationContext>>> {
        Arc::new(RwLock::new(
            Box::new(StandaloneApplicationContext::default()) as Box<dyn ApplicationContext>
        ))
    }

    /// Returns an HTTP security a configurer can be applied to.
    fn http_security() -> HttpSecurity {
        HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new())
    }

    /// Returns the registration the tests are performed for.
    fn registration() -> RelyingPartyRegistration {
        RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("sp-entity-id")
            .assertion_consumer_service_location("https://sp.example.org/login/saml2/sso/okta")
            .asserting_party_entity_id("idp-entity-id")
            .single_sign_on_service_location("https://idp.example.org/sso")
            .build()
    }

    /// Returns a repository over one registration.
    fn repository() -> Arc<InMemoryRelyingPartyRegistrationRepository> {
        Arc::new(InMemoryRelyingPartyRegistrationRepository::new(vec![
            registration(),
        ]))
    }

    /// A signature verifier that believes every signature.
    struct AcceptingSignature;

    impl crate::saml2::login::SignatureVerifier for AcceptingSignature {
        fn verify(
            &self,
            _registration: &RelyingPartyRegistration,
            _response: &Saml2Response,
        ) -> Result<(), Saml2Error> {
            Ok(())
        }
    }

    /// Returns a response that holds none of the conditions.
    fn minimal_response() -> Saml2Response {
        Saml2Response::parse(
            r#"<saml2p:Response xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol"/>"#,
        )
        .expect("the response parses")
    }

    #[test]
    fn without_a_signature_verifier_every_response_is_refused() {
        let configurer = Saml2LoginConfigurer::<HttpSecurity>::new(context());

        let error = configurer
            .build_verifier()
            .verify(&registration(), &minimal_response())
            .expect_err("no verifier is configured");

        assert!(
            error.message().contains("no assertion verifier"),
            "the error names the missing verifier, was: {}",
            error.message()
        );
    }

    #[test]
    fn a_signature_verifier_assembles_the_strict_checks() {
        let mut configurer = Saml2LoginConfigurer::<HttpSecurity>::new(context());
        configurer.signature_verifier(Arc::new(AcceptingSignature));

        let error = configurer
            .build_verifier()
            .verify(&registration(), &minimal_response())
            .expect_err("the conditions of the response do not hold");

        assert!(
            !error.message().contains("no assertion verifier"),
            "the strict verifier ran instead of the rejecting one, was: {}",
            error.message()
        );
    }

    /// Returns a user details service.
    fn user_details_service() -> Arc<dyn UserDetailsService> {
        Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
            User::with_username("Max")
                .password(Some("password".to_string()))
                .build(),
        )]))
    }

    /// A verifier that believes every response.
    struct AcceptingVerifier;

    impl AssertionVerifier for AcceptingVerifier {
        fn verify(
            &self,
            _registration: &RelyingPartyRegistration,
            _response: &Saml2Response,
        ) -> Result<(), Saml2Error> {
            Ok(())
        }
    }

    /// Returns a configurer that is ready to be applied.
    fn configurer() -> Saml2LoginConfigurer<HttpSecurity> {
        let mut configurer = Saml2LoginConfigurer::<HttpSecurity>::new(context());
        configurer.relying_party_registration_repository(repository());
        configurer.user_details_service(user_details_service());
        configurer.assertion_verifier(Arc::new(AcceptingVerifier));

        configurer
    }

    #[test]
    fn the_login_registers_both_endpoints() {        let mut http = http_security();
        let mut configurer = configurer();

        configurer.init(&mut http);
        configurer.configure(&mut http);

        let names = http
            .build()
            .get_filters()
            .iter()
            .map(|filter| filter.name().to_owned())
            .collect::<Vec<_>>();
        assert!(
            names
                .iter()
                .any(|name| name == "Saml2AuthenticationRequestFilter"),
            "the endpoints were: {names:?}"
        );
        assert!(
            names.iter().any(|name| name == "Saml2AuthenticationFilter"),
            "the endpoints were: {names:?}"
        );
    }

    #[test]
    #[should_panic(expected = "relyingPartyRegistrationRepository cannot be null")]
    fn a_missing_repository_is_reported() {
        let mut http = http_security();
        let mut configurer = Saml2LoginConfigurer::<HttpSecurity>::new(context());
        configurer.user_details_service(user_details_service());

        configurer.init(&mut http);
    }

    #[test]
    #[should_panic(expected = "A UserDetailsService is required to enable saml2Login()")]
    fn a_missing_user_details_service_is_reported() {
        let mut http = http_security();
        let mut configurer = Saml2LoginConfigurer::<HttpSecurity>::new(context());
        configurer.relying_party_registration_repository(repository());

        configurer.init(&mut http);
    }

    /// The response an asserting party posts back for the test registration.
    const RESPONSE: &str = r#"<saml2p:Response xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion"><saml2:Issuer>idp-entity-id</saml2:Issuer><saml2p:Status><saml2p:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></saml2p:Status><saml2:Assertion><saml2:Subject><saml2:NameID>Max</saml2:NameID></saml2:Subject></saml2:Assertion></saml2p:Response>"#;

    /// Runs the filters of a chain in order, the way the chain proxy does.
    #[derive(Clone)]
    struct Chain {
        filters: Arc<Vec<Arc<dyn next_web_core::http::filter::HttpFilter>>>,
        index: usize,
    }

    impl Chain {
        /// Returns a chain that starts at the first filter.
        fn new(filters: Arc<Vec<Arc<dyn next_web_core::http::filter::HttpFilter>>>) -> Self {
            Self { filters, index: 0 }
        }

        /// Runs the filter at the current index and everything after it.
        async fn run(
            &self,
            request: &mut dyn next_web_core::http::HttpRequest,
            response: &mut dyn next_web_core::http::HttpResponse,
        ) -> Result<(), next_web_core::http::filter::FilterError> {
            next_web_core::http::filter::HttpFilterChain::do_filter(self, request, response).await
        }
    }

    #[next_web_core::async_trait]
    impl next_web_core::http::filter::HttpFilterChain for Chain {
        async fn do_filter(
            &self,
            request: &mut dyn next_web_core::http::HttpRequest,
            response: &mut dyn next_web_core::http::HttpResponse,
        ) -> Result<(), next_web_core::http::filter::FilterError> {
            let Some(filter) = self.filters.get(self.index) else {
                return Ok(());
            };
            let next = Self {
                filters: self.filters.clone(),
                index: self.index + 1,
            };

            filter.do_filter(request, response, &next).await
        }
    }

    /// Returns the assertion consumer post of the given response.
    fn acs_request(registration_id: &str, document: &str) -> axum::extract::Request {
        use base64::Engine;

        let encoded = base64::engine::general_purpose::STANDARD.encode(document.as_bytes());
        let body = format!("SAMLResponse={}", urlencoding::encode(&encoded));
        let mut request = axum::extract::Request::builder()
            .method("POST")
            .uri(format!(
                "https://sp.example.org/login/saml2/sso/{registration_id}"
            ))
            .body(axum::body::Body::empty())
            .expect("a valid request");
        request
            .extensions_mut()
            .insert(next_web_core::http::RequestBody::new(body.into_bytes()));

        request
    }

    /// Returns the name of the authenticated principal of a request, if one was
    /// stored.
    fn authenticated_username(request: &dyn next_web_core::http::HttpRequest) -> Option<String> {
        use next_web_core::any::AnyValue;

        let AnyValue::BoxedValue(value) = request.get_attribute(
            crate::web::context::RequestAttributeSecurityContextRepository::
                DEFAULT_REQUEST_ATTR_NAME,
        )?
        else {
            return None;
        };
        let context = (**value)
            .as_any()
            .downcast_ref::<Arc<dyn crate::core::context::SecurityContext>>()?;

        context
            .get_authentication()
            .and_then(|authentication| authentication.principal().map(|p| p.to_string()))
    }

    /// Returns the `Location` header of a response.
    fn redirect_of(response: &axum::response::Response) -> Option<String> {
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }

    /// Returns the type of the authentication the request stored, if one was
    /// stored.
    fn authenticated_type(
        request: &dyn next_web_core::http::HttpRequest,
    ) -> Option<std::any::TypeId> {
        use next_web_core::any::AnyValue;

        let AnyValue::BoxedValue(value) = request.get_attribute(
            crate::web::context::RequestAttributeSecurityContextRepository::
                DEFAULT_REQUEST_ATTR_NAME,
        )?
        else {
            return None;
        };
        let context = (**value)
            .as_any()
            .downcast_ref::<Arc<dyn crate::core::context::SecurityContext>>()?;

        context
            .get_authentication()
            .map(|authentication| authentication.of())
    }

    #[tokio::test]
    async fn a_believed_response_authenticates_through_the_chain() {
        let mut http = http_security();
        http.with(configurer(), |_| {});
        let chain = http.build();
        let filters = Arc::new(chain.get_filters().to_vec());

        let mut request = acs_request("okta", RESPONSE);
        let mut response = axum::response::Response::default();
        Chain::new(filters)
            .run(&mut request, &mut response)
            .await
            .expect("the chain runs");

        assert!(
            response.status().is_redirection(),
            "the login redirects, status was {}",
            response.status()
        );
        assert_eq!(redirect_of(&response), Some("/".to_owned()));
        assert_eq!(authenticated_username(&request).as_deref(), Some("Max"));
        assert_eq!(
            authenticated_type(&request),
            Some(std::any::TypeId::of::<crate::saml2::login::Saml2Authentication>()),
            "the stored authentication is the SAML 2.0 one, not a rebuilt plain one"
        );
    }

    #[tokio::test]
    async fn a_rejected_response_redirects_back_to_the_login_page() {
        let mut http = http_security();
        let mut configurer = Saml2LoginConfigurer::<HttpSecurity>::new(context());
        configurer.relying_party_registration_repository(repository());
        configurer.user_details_service(user_details_service());
        configurer.assertion_verifier(Arc::new(RejectingAssertionVerifier));
        http.with(configurer, |_| {});
        let chain = http.build();
        let filters = Arc::new(chain.get_filters().to_vec());

        let mut request = acs_request("okta", RESPONSE);
        let mut response = axum::response::Response::default();
        Chain::new(filters)
            .run(&mut request, &mut response)
            .await
            .expect("the chain runs");

        assert_eq!(
            redirect_of(&response),
            Some("/login?error".to_owned()),
            "a refused login sends the user back"
        );
    }
}
