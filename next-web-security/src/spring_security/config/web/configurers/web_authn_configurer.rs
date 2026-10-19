use std::{
    collections::BTreeSet,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
};

use next_web_context::ApplicationContextExt;
use next_web_core::{ApplicationContext, Required};

use crate::{
    authorization::AuthenticationManager,
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    core::userdetails::UserDetailsService,
    webauthn::{
        ChallengeRepository, InMemoryChallengeRepository, InMemoryUserCredentialRepository,
        RelyingParty, UserCredentialRepository, WebAuthnManager,
    },
    web::{
        authentication::{
            ui::DefaultLoginPageGeneratingFilter, BasicAuthenticationFilter,
            SimpleUrlAuthenticationFailureHandler,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
        webauthn::{
            WebAuthnAuthenticationFilter, WebAuthnAuthenticationProvider,
            WebAuthnRegistrationFilter,
        },
    },
};

/// Configures Web Authentication.
///
/// The configurer registers the four endpoints of a ceremony, and the provider
/// that turns a verified assertion into a login. The closure configures the
/// relying party the application presents itself as — its identifier, its name,
/// and the origins a ceremony may run on.
///
/// A relying party that allows no origin refuses every ceremony, and a ceremony
/// without a relying party identifier has nothing to bind a credential to, so
/// both are reported when the chain is built rather than when a login fails.
#[derive(Clone)]
pub struct WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    base: BaseHttpConfigurer<Self, H>,
    rp_id: Option<String>,
    rp_name: Option<String>,
    allowed_origins: BTreeSet<String>,
    disable_default_registration_page: bool,
    require_user_verification: bool,
    user_details_service: Option<Arc<dyn UserDetailsService>>,
    credential_repository: Option<Arc<dyn UserCredentialRepository>>,
    challenge_repository: Option<Arc<dyn ChallengeRepository>>,
    manager: Option<Arc<WebAuthnManager>>,
    context: Arc<RwLock<Box<dyn ApplicationContext>>>,
}

impl<H> WebAuthnConfigurer<H>
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

    /// The Relying Party id.
    pub fn rp_id(&mut self, value: impl Into<String>) -> &mut Self {
        self.rp_id = Some(value.into());
        self
    }

    /// Sets the relying party name
    pub fn rp_name(&mut self, value: impl Into<String>) -> &mut Self {
        self.rp_name = Some(value.into());
        self
    }

    /// Convenience method for allowedOrigins(Set)
    pub fn allowed_origins<I, S>(&mut self, origins: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_origins = origins.into_iter().map(Into::into).collect();
        self
    }

    /// Configures whether the default webauthn registration should be disabled. Setting it to true will
    /// prevent the configurer from registering the DefaultWebAuthnRegistrationPageGeneratingFilter.
    pub fn disable_default_registration_page(&mut self, disable: bool) -> &mut Self {
        self.disable_default_registration_page = disable;
        self
    }

    /// Sets whether a ceremony requires the authenticator to verify the user.
    pub fn require_user_verification(&mut self, require: bool) -> &mut Self {
        self.require_user_verification = require;
        self
    }

    /// Sets the user details service the user of a credential is loaded with.
    pub fn user_details_service(
        &mut self,
        user_details_service: Arc<dyn UserDetailsService>,
    ) -> &mut Self {
        self.user_details_service = Some(user_details_service);
        self
    }

    /// Sets where the credentials are kept.
    pub fn credential_repository(
        &mut self,
        credential_repository: Arc<dyn UserCredentialRepository>,
    ) -> &mut Self {
        self.credential_repository = Some(credential_repository);
        self
    }

    /// Sets where the challenges a ceremony answers are remembered.
    pub fn challenge_repository(
        &mut self,
        challenge_repository: Arc<dyn ChallengeRepository>,
    ) -> &mut Self {
        self.challenge_repository = Some(challenge_repository);
        self
    }

    /// Returns the manager the endpoints of the chain run ceremonies with.
    pub fn web_authn_manager(&self) -> Option<Arc<WebAuthnManager>> {
        self.manager.clone()
    }

    /// Resolves the user details service a credential's user is loaded with.
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn init(&mut self, http: &mut H) {
        if let Some(id) = &self.rp_id {
            assert!(!id.trim().is_empty(), "rp_id cannot be empty");
        }

        let Some(rp_id) = self.rp_id.clone() else {
            panic!(
                "webAuthn() requires the identifier of the relying party: pass it to the DSL with \
                 rpId(\"example.org\")"
            );
        };
        let Some(user_details_service) = self.resolve_user_details_service() else {
            panic!(
                "A UserDetailsService is required to enable webAuthn(). Please provide it as a bean \
                 or pass it to the webAuthn() DSL."
            );
        };

        let mut manager = WebAuthnManager::new(RelyingParty::new(
            rp_id.clone(),
            self.rp_name.clone().unwrap_or(rp_id),
        ));
        manager.allowed_origins(self.allowed_origins.iter().cloned());
        manager.require_user_verification(self.require_user_verification);
        manager.credential_repository(self.credential_repository.clone().unwrap_or_else(|| {
            Arc::new(InMemoryUserCredentialRepository::default()) as Arc<dyn UserCredentialRepository>
        }));
        manager.challenge_repository(self.challenge_repository.clone().unwrap_or_else(|| {
            Arc::new(InMemoryChallengeRepository::default()) as Arc<dyn ChallengeRepository>
        }));

        let manager = Arc::new(manager);
        http.authentication_provider(Arc::new(WebAuthnAuthenticationProvider::new(
            manager.clone(),
            user_details_service,
        )));
        self.manager = Some(manager);

        // The default login page offers the passkey sign-in, and the script that
        // runs the ceremony, only for a chain that configured a relying party.
        // The script submits JSON, so the page has to carry the headers of the
        // request — the CSRF token among them — for the submissions to be
        // accepted when CSRF protection is enabled.
        if let Some(login_page) = http.shared_object_mut::<DefaultLoginPageGeneratingFilter>() {
            if login_page.is_enabled() {
                login_page.set_passkeys_enabled(true);
                login_page.set_resolve_headers(|request| {
                    let token = request
                        .get_attribute(std::any::type_name::<&dyn crate::web::csrf::CsrfToken>())
                        .and_then(|value| {
                            value.as_ref_value::<Arc<dyn crate::web::csrf::CsrfToken>>()
                        });

                    match token {
                        Some(token) => std::collections::HashMap::from([(
                            token.header_name().to_string(),
                            token.token().to_string(),
                        )]),
                        None => std::collections::HashMap::new(),
                    }
                });
            }
        }
    }

    fn configure(&mut self, http: &mut H) {
        if !self.disable_default_registration_page {
            http.add_filter(crate::web::authentication::ui::DefaultResourcesFilter::webauthn());
            // The page a signed-in user registers a passkey from. It answers a
            // GET; the POST of the same path is the ceremony below.
            http.add_filter_before::<
                crate::web::authentication::ui::DefaultWebAuthnRegistrationPageGeneratingFilter,
                BasicAuthenticationFilter,
            >(
                crate::web::authentication::ui::DefaultWebAuthnRegistrationPageGeneratingFilter::new(
                ),
            );
        }

        let Some(manager) = self.manager.clone() else {
            // `init` is where a chain without a relying party is reported, so a
            // configurer that was not initialized installs no endpoint.
            return;
        };

        // Both pairs of endpoints sit ahead of the basic authentication filter,
        // the point of the chain the other browser logins are processed at.
        http.add_filter_before::<WebAuthnRegistrationFilter, BasicAuthenticationFilter>(
            WebAuthnRegistrationFilter::new(manager.clone()),
        );

        let mut filter = WebAuthnAuthenticationFilter::new(manager);
        if let Some(authentication_manager) = http.shared_object::<Arc<dyn AuthenticationManager>>() {
            filter.set_authentication_manager(authentication_manager.clone());
        }
        // A login has to outlive the request that made it. The context
        // repository of the chain is the one the security context configurer
        // built — the request attribute and the session — and a filter that
        // saves the context anywhere else leaves the next request anonymous.
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

        http.add_filter_before::<WebAuthnAuthenticationFilter, BasicAuthenticationFilter>(filter);
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>> for WebAuthnConfigurer<H>
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

impl<H> Deref for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<H> Default for WebAuthnConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            base: Default::default(),
            rp_id: None,
            rp_name: None,
            allowed_origins: BTreeSet::new(),
            disable_default_registration_page: false,
            require_user_verification: false,
            user_details_service: None,
            credential_repository: None,
            challenge_repository: None,
            manager: None,
            context: Arc::new(RwLock::new(
                Box::new(super::super::builders::StandaloneApplicationContext::default())
                    as Box<dyn ApplicationContext>,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        config::{
            authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
            web::builders::{HttpSecurity, StandaloneApplicationContext},
        },
        core::userdetails::User,
        provisioning::InMemoryUserDetailsManager,
        web::{
            authentication::ui::DefaultLoginPageGeneratingFilter, security_filter_chain::SecurityFilterChain,
        },
    };

    /// Returns a context that contributes no singletons.
    fn context() -> Arc<RwLock<Box<dyn ApplicationContext>>> {
        Arc::new(RwLock::new(
            Box::new(StandaloneApplicationContext::default()) as Box<dyn ApplicationContext>,
        ))
    }

    /// Returns an HTTP security a configurer can be applied to.
    fn http_security() -> HttpSecurity {
        HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new())
    }

    /// Returns a user details service that holds `Max`.
    fn user_details_service() -> Arc<dyn UserDetailsService> {
        Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
            User::with_username("Max")
                .password(Some("password".to_string()))
                .build(),
        )]))
    }

    /// Returns a configurer the tests can apply.
    fn configurer() -> WebAuthnConfigurer<HttpSecurity> {
        let mut configurer = WebAuthnConfigurer::new(context());
        configurer.rp_id("sp.example.org");
        configurer.rp_name("The application");
        configurer.allowed_origins(["https://sp.example.org"]);
        configurer.user_details_service(user_details_service());

        configurer
    }

    #[test]
    fn the_configuration_registers_both_ceremonies() {
        let mut http = http_security();
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
            names.iter().any(|name| name == "WebAuthnRegistrationFilter"),
            "the endpoints were: {names:?}"
        );
        assert!(
            names.iter().any(|name| name == "WebAuthnAuthenticationFilter"),
            "the endpoints were: {names:?}"
        );
    }

    #[test]
    fn the_configuration_enables_the_passkey_form_of_the_login_page() {
        let mut http = http_security();
        let mut login_page = DefaultLoginPageGeneratingFilter::default();
        // The page exists only for an application that configured a login.
        login_page.set_form_login_enabled(true);
        http.set_shared_object(login_page);
        let mut configurer = configurer();

        configurer.init(&mut http);

        let page = http
            .shared_object::<DefaultLoginPageGeneratingFilter>()
            .expect("the login page is a shared object");
        assert!(
            page.passkeys_enabled(),
            "the form and the script are rendered only when a relying party is configured"
        );
    }

    #[test]
    fn the_configuration_serves_the_registration_page() {
        let mut http = http_security();
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
                .any(|name| name == "DefaultWebAuthnRegistrationPageGeneratingFilter"),
            "a signed-in user registers a passkey from a page, the endpoints were: {names:?}"
        );
    }

    #[test]
    fn the_registration_page_can_be_disabled() {
        let mut http = http_security();
        let mut configurer = configurer();
        configurer.disable_default_registration_page(true);
        configurer.init(&mut http);
        configurer.configure(&mut http);

        let names = http
            .build()
            .get_filters()
            .iter()
            .map(|filter| filter.name().to_owned())
            .collect::<Vec<_>>();

        assert!(
            !names
                .iter()
                .any(|name| name == "DefaultWebAuthnRegistrationPageGeneratingFilter"),
            "the page is not served when the application has its own, the endpoints were: {names:?}"
        );
    }

    #[test]
    #[should_panic(expected = "webAuthn() requires the identifier of the relying party")]
    fn a_missing_relying_party_id_is_reported() {
        let mut http = http_security();
        let mut configurer = WebAuthnConfigurer::<HttpSecurity>::new(context());
        configurer.user_details_service(user_details_service());

        configurer.init(&mut http);
    }

    #[test]
    #[should_panic(expected = "A UserDetailsService is required to enable webAuthn()")]
    fn a_missing_user_details_service_is_reported() {
        let mut http = http_security();
        let mut configurer = WebAuthnConfigurer::<HttpSecurity>::new(context());
        configurer.rp_id("sp.example.org");

        configurer.init(&mut http);
    }

    /// Signs the security context in as the given user.
    fn signed_in_as(user_name: &str) {
        use crate::core::context::{SecurityContextHolder, SecurityContextImpl};

        let authentication: Arc<dyn crate::core::Authentication> = Arc::new(
            crate::web::webauthn::WebAuthnAuthentication::new(user_name, Vec::new(), Vec::new()),
        );
        SecurityContextHolder::set_context(Arc::new(SecurityContextImpl::new(authentication)));
    }

    /// Returns a `POST` with the given body.
    fn post(path: &str, body: &[u8]) -> axum::extract::Request {
        use next_web_core::http::RequestBody;

        let mut request = axum::extract::Request::builder()
            .method("POST")
            .uri(format!("https://sp.example.org{path}"))
            .body(axum::body::Body::empty())
            .expect("a valid request");
        request
            .extensions_mut()
            .insert(RequestBody::new(body.to_vec()));

        request
    }

    /// Runs the given filters in order over the request.
    async fn run_chain(
        filters: &Arc<Vec<Arc<dyn next_web_core::http::filter::HttpFilter>>>,
        request: &mut axum::extract::Request,
    ) -> axum::response::Response {
        let mut response = axum::response::Response::default();
        crate::web::test_support::Chain::new(filters.clone())
            .run(request, &mut response)
            .await
            .expect("the chain runs");

        response
    }

    /// Returns the JSON body of a response.
    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body is collected");

        serde_json::from_slice(&body).expect("the body is JSON")
    }

    /// Returns the challenge of a response of the options endpoints.
    fn challenge_of(options: &serde_json::Value) -> Vec<u8> {
        crate::webauthn::test_support::base64_url(
            options["challenge"]
                .as_str()
                .expect("the options carry a challenge"),
        )
    }

    /// Returns the name of the authenticated principal of a request.
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

    #[tokio::test]
    async fn a_passkey_registers_and_then_logs_the_user_in_through_the_chain() {
        use crate::{
            core::context::SecurityContextHolder,
            webauthn::{
                test_support::{
                    attestation_object, authenticator_data, authentication_submission,
                    client_data_json, cose_key_of, registration_submission, sha256, sign,
                    signing_key, ATTESTED_CREDENTIAL_DATA, ORIGIN, RP_ID, USER_PRESENT,
                },
                InMemoryUserCredentialRepository, UserCredentialRepository, CREATE, GET,
            },
        };

        let credentials = Arc::new(InMemoryUserCredentialRepository::default());
        let mut http = http_security();
        let mut configurer = configurer();
        configurer.credential_repository(credentials.clone());
        http.with(configurer, |_| {});
        let chain = http.build();
        let filters = Arc::new(
            chain
                .get_filters()
                .iter()
                .filter(|filter| {
                    matches!(
                        filter.name(),
                        "WebAuthnRegistrationFilter" | "WebAuthnAuthenticationFilter"
                    )
                })
                .cloned()
                .collect::<Vec<_>>(),
        );
        assert_eq!(filters.len(), 2, "both ceremonies are wired");

        // A registration runs for the user the request is authenticated as.
        signed_in_as("Max");
        let mut request = post("/webauthn/register/options", b"{}");
        let options = body_json(run_chain(&filters, &mut request).await).await;
        let challenge = challenge_of(&options);
        let cose_key = cose_key_of(&signing_key());
        let registration_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(CREATE, &challenge, ORIGIN, false),
            &attestation_object("none", &[], &registration_data),
        );
        let mut request = post("/webauthn/register", &submission);
        let response = run_chain(&filters, &mut request).await;
        let status = response.status();
        SecurityContextHolder::clear_context();

        assert_eq!(status, 200, "the registration is taken");
        assert!(
            credentials
                .find_by_credential_id(b"the-credential")
                .is_some(),
            "the credential is stored"
        );

        // The login, with the credential that was just registered.
        let mut request = post("/webauthn/authenticate/options", br#"{"username":"Max"}"#);
        let options = body_json(run_chain(&filters, &mut request).await).await;
        let challenge = challenge_of(&options);
        let client_data = client_data_json(GET, &challenge, ORIGIN, false);
        let assertion_data = authenticator_data(RP_ID, USER_PRESENT, 1, None);
        let mut message = assertion_data.clone();
        message.extend_from_slice(&sha256(&client_data));
        let submission = authentication_submission(
            b"the-credential",
            &client_data,
            &assertion_data,
            &sign(&signing_key(), &message),
            None,
        );
        let mut request = post("/webauthn/authenticate", &submission);
        let response = run_chain(&filters, &mut request).await;

        assert!(
            response.status().is_redirection(),
            "the login redirects, status was {}",
            response.status()
        );
        assert_eq!(
            authenticated_username(&request).as_deref(),
            Some("Max"),
            "the principal is the user of the credential"
        );
    }
}
