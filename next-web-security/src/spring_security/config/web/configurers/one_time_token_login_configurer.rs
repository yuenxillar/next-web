use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_context::ApplicationContextExt;
use next_web_core::{any::AnyValue, http::HttpMethod, http::HttpRequest, ApplicationContext};

use crate::{
    authentication::{
        ott::{
            in_memory_one_time_token_service::InMemoryOneTimeTokenService,
            one_time_token_authentication_provider::OneTimeTokenAuthenticationProvider,
            one_time_token_service::OneTimeTokenService,
        },
        AuthenticationProvider,
    },
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        web::{
            configurers::{
                BaseAuthenticationFilterConfigurer, BaseAuthenticationFilterConfigurerExt,
                ErrorHandlingConfigurer,
            },
            http_security_builder::HttpSecurityBuilder,
        },
    },
    core::authority::FactorGrantedAuthority,
    web::{
        authentication::{
            ott::{
                DefaultGenerateOneTimeTokenRequestResolver, GenerateOneTimeTokenFilter,
                GenerateOneTimeTokenRequestResolver, OneTimeTokenAuthenticationFilter,
                OneTimeTokenGenerationSuccessHandler,
            },
            ui::{
                DefaultLoginPageGeneratingFilter, DefaultOneTimeTokenSubmitPageGeneratingFilter,
                DefaultResourcesFilter,
            },
        },
        csrf::CsrfToken,
        default_security_filter_chain::DefaultSecurityFilterChain,
        util::matcher::{RequestMatcher, DEFAULT_BUILDER},
    },
};

/// The default URL used to generate a one-time token.
const DEFAULT_GENERATE_URL: &str = "/ott/generate";

/// The default URL used to render the one-time token submit page.
const DEFAULT_SUBMIT_PAGE_URL: &str = "/login/ott";

/// Configures One-Time Token (OTT) Login.
///
/// One-Time Token Login allows users to authenticate by obtaining a single-use
/// token out of band (for example, by email). The configurer registers the
/// following filters:
///
/// * `GenerateOneTimeTokenFilter` - handles token generation requests.
/// * `OneTimeTokenAuthenticationFilter` - authenticates using the submitted token.
/// * `DefaultOneTimeTokenSubmitPageGeneratingFilter` - renders the submit page.
///
/// It also registers an `OneTimeTokenAuthenticationProvider` and integrates with
/// `DefaultLoginPageGeneratingFilter` so that a default login page can link to
/// token generation.
///
/// The only required configuration is a `OneTimeTokenGenerationSuccessHandler`,
/// which may be provided through the DSL or as a bean in the `ApplicationContext`.
#[derive(Clone)]
pub struct OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    one_time_token_service: Option<Arc<dyn OneTimeTokenService>>,
    token_generating_url: String,
    one_time_token_generation_success_handler:
        Option<Arc<dyn OneTimeTokenGenerationSuccessHandler>>,
    authentication_provider: Option<Arc<dyn AuthenticationProvider>>,
    user_details_service: Option<Arc<dyn crate::core::userdetails::UserDetailsService>>,
    request_resolver: Option<Arc<dyn GenerateOneTimeTokenRequestResolver>>,

    default_submit_page_url: String,
    submit_page_enabled: bool,
    custom_login_page: Option<Box<str>>,

    base: BaseAuthenticationFilterConfigurer<H, Self, OneTimeTokenAuthenticationFilter>,
}

impl<H> OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    /// Creates a new `OneTimeTokenLoginConfigurer`.
    ///
    /// # Arguments
    ///
    /// * `context` - the context used to resolve beans such as the
    ///   `OneTimeTokenService` and the
    ///   `OneTimeTokenGenerationSuccessHandler`.
    ///
    /// The beans are resolved right away: the configurer does not keep the
    /// context itself, and a value set through the DSL still takes precedence.
    pub fn new(context: Arc<std::sync::RwLock<Box<dyn ApplicationContext>>>) -> Self {
        let mut context = context
            .write()
            .expect("the application context lock is poisoned");
        let context: &mut dyn ApplicationContext = &mut **context;

        let mut configurer = Self::default();

        if configurer.one_time_token_service.is_none() {
            configurer.one_time_token_service = context
                .get_singleton_option::<Arc<dyn OneTimeTokenService>>()
                .cloned();
        }

        if configurer
            .one_time_token_generation_success_handler
            .is_none()
        {
            configurer.one_time_token_generation_success_handler = context
                .get_singleton_option::<Arc<dyn OneTimeTokenGenerationSuccessHandler>>()
                .cloned();
        }

        if configurer.request_resolver.is_none() {
            configurer.request_resolver = context
                .get_singleton_option::<Arc<dyn GenerateOneTimeTokenRequestResolver>>()
                .cloned();
        }

        if configurer.user_details_service.is_none() {
            configurer.user_details_service = context
                .get_singleton_option::<Arc<dyn crate::core::userdetails::UserDetailsService>>()
                .cloned();
        }

        configurer
    }

    /// Specifies the `AuthenticationProvider` to use when authenticating the user.
    pub fn authentication_provider(
        &mut self,
        authentication_provider: Arc<dyn AuthenticationProvider>,
    ) -> &mut Self {
        self.authentication_provider = Some(authentication_provider);
        self
    }

    /// Sets the user details service the one-time token authentication uses to
    /// load the user of a token.
    ///
    /// A value set here takes precedence over the one resolved from the
    /// application context or the shared objects of the HTTP security.
    ///
    /// # Arguments
    ///
    /// * `user_details_service` - The service that loads the user of a token.
    pub fn user_details_service(
        &mut self,
        user_details_service: Arc<dyn crate::core::userdetails::UserDetailsService>,
    ) -> &mut Self {
        self.user_details_service = Some(user_details_service);
        self
    }

    /// Returns the user details service of the configurer, if one is set.
    pub fn get_user_details_service(
        &self,
    ) -> Option<&Arc<dyn crate::core::userdetails::UserDetailsService>> {
        self.user_details_service.as_ref()
    }

    /// Specifies the URL that a one-time token generation request will be processed
    /// on. Defaults to `/ott/generate`.
    ///
    /// # Panics
    ///
    /// Panics when the URL is empty or whitespace.
    pub fn token_generating_url(&mut self, token_generating_url: &str) -> &mut Self {
        assert_has_text(token_generating_url, "tokenGeneratingUrl");
        self.token_generating_url = token_generating_url.to_string();
        self
    }

    /// Specifies the strategy to be used to handle generated one-time tokens.
    pub fn token_generation_success_handler(
        &mut self,
        one_time_token_generation_success_handler: Arc<dyn OneTimeTokenGenerationSuccessHandler>,
    ) -> &mut Self {
        self.one_time_token_generation_success_handler =
            Some(one_time_token_generation_success_handler);
        self
    }

    /// Configures whether the default one-time token submit page should be shown.
    /// This prevents the `DefaultOneTimeTokenSubmitPageGeneratingFilter` from being
    /// configured when set to `false`.
    pub fn show_default_submit_page(&mut self, show: bool) -> &mut Self {
        self.submit_page_enabled = show;
        self
    }

    /// Sets the URL that the default submit page will be generated on. Defaults to
    /// `/login/ott`. This also enables the default submit page.
    ///
    /// # Panics
    ///
    /// Panics when the URL is empty or whitespace.
    pub fn default_submit_page_url(&mut self, submit_page_url: &str) -> &mut Self {
        assert_has_text(submit_page_url, "submitPageUrl");
        self.default_submit_page_url = submit_page_url.to_string();
        self.show_default_submit_page(true);
        self
    }

    /// Specifies the URL that the submit page posts a token to. Defaults to
    /// `/login/ott`.
    ///
    /// Only `POST` requests are processed, so a submission has to carry a valid
    /// CSRF token while CSRF protection is enabled.
    ///
    /// # Panics
    ///
    /// Panics when the URL is empty or whitespace.
    pub fn login_processing_url(&mut self, login_processing_url: &str) -> &mut Self {
        assert_has_text(login_processing_url, "loginProcessingUrl");
        <Self as BaseAuthenticationFilterConfigurerExt<H>>::login_processing_url(
            self,
            login_processing_url,
        );
        self
    }

    /// Specifies the URL a user is sent to when a login is required.
    ///
    /// The page replaces the generated login page, and the entry point of the
    /// chain redirects to it. It is applied when the configurer is initialized,
    /// because updating the defaults of the chain needs the builder.
    pub fn login_page(&mut self, login_page: &str) -> &mut Self {
        self.custom_login_page = Some(login_page.into());
        self
    }

    /// Configures the `OneTimeTokenService` used to generate and consume tokens.
    pub fn token_service(
        &mut self,
        one_time_token_service: Arc<dyn OneTimeTokenService>,
    ) -> &mut Self {
        self.one_time_token_service = Some(one_time_token_service);
        self
    }

    /// Configures the `GenerateOneTimeTokenRequestResolver` used to resolve a
    /// `GenerateOneTimeTokenRequest` from the `HttpRequest`. Defaults to
    /// `DefaultGenerateOneTimeTokenRequestResolver`.
    pub fn generate_request_resolver(
        &mut self,
        request_resolver: Arc<dyn GenerateOneTimeTokenRequestResolver>,
    ) -> &mut Self {
        self.request_resolver = Some(request_resolver);
        self
    }

    /// Configures the `AuthenticationConverter` used by the authentication filter.
    /// Defaults to `OneTimeTokenAuthenticationConverter`.
    pub fn authentication_converter(
        &mut self,
        authentication_converter: Arc<dyn crate::web::authentication::AuthenticationConverter>,
    ) -> &mut Self {
        if let Some(filter) = self.base.get_authentication_filter_mut() {
            filter.set_authentication_converter(authentication_converter);
        }
        self
    }

    /// Resolves the `OneTimeTokenService` to use, or creates the default one.
    ///
    /// The resolved service is kept on the configurer, because the generate filter
    /// writes tokens into it and the authentication provider reads them from it:
    /// two instances would mean a token that is issued can never be redeemed.
    fn get_one_time_token_service(&mut self) -> Arc<dyn OneTimeTokenService> {
        if let Some(service) = &self.one_time_token_service {
            return service.clone();
        }

        let service: Arc<dyn OneTimeTokenService> = Arc::new(InMemoryOneTimeTokenService::new());
        self.one_time_token_service = Some(service.clone());

        service
    }

    /// Returns the `OneTimeTokenGenerationSuccessHandler` to use.
    ///
    /// # Panics
    ///
    /// Panics when the DSL did not set one and the application does not
    /// contribute one, because nothing else can deliver a generated token to the
    /// user.
    fn get_one_time_token_generation_success_handler(
        &mut self,
    ) -> Arc<dyn OneTimeTokenGenerationSuccessHandler> {
        let Some(handler) = &self.one_time_token_generation_success_handler else {
            panic!(
                "A OneTimeTokenGenerationSuccessHandler is required to enable \
                 oneTimeTokenLogin().\nPlease provide it as a bean or pass it to the \
                 oneTimeTokenLogin() DSL."
            );
        };

        handler.clone()
    }

    /// Resolves the `GenerateOneTimeTokenRequestResolver` to use.
    ///
    /// The resolver is created once for the same reason the token service is: it
    /// is a collaborator of the filter the configurer registers.
    fn get_generate_request_resolver(&mut self) -> Arc<dyn GenerateOneTimeTokenRequestResolver> {
        if let Some(resolver) = &self.request_resolver {
            return resolver.clone();
        }

        let resolver: Arc<dyn GenerateOneTimeTokenRequestResolver> =
            Arc::new(DefaultGenerateOneTimeTokenRequestResolver::default());
        self.request_resolver = Some(resolver.clone());

        resolver
    }

    /// Builds the `AuthenticationProvider` used to authenticate one-time tokens.
    ///
    /// # Panics
    ///
    /// Panics when no user details service was set through the DSL, shared by the
    /// application, or contributed to the application context, because the
    /// provider loads the user of a token with it.
    fn get_authentication_provider(&mut self) -> Arc<dyn AuthenticationProvider> {
        if let Some(provider) = &self.authentication_provider {
            return provider.clone();
        }

        let Some(user_details_service) = self.user_details_service.clone() else {
            panic!(
                "A UserDetailsService is required to enable oneTimeTokenLogin(). \
                 Please provide it as a bean or pass it to the oneTimeTokenLogin() DSL."
            );
        };

        let provider: Arc<dyn AuthenticationProvider> =
            Arc::new(OneTimeTokenAuthenticationProvider::new(
                self.get_one_time_token_service(),
                user_details_service,
            ));
        self.authentication_provider = Some(provider.clone());

        provider
    }

    /// Configures the `DefaultLoginPageGeneratingFilter` shared object so that a
    /// generated login page links to one-time token generation.
    fn init_default_login_filter(&mut self, http: &mut H) {
        let Some(login_page_generating_filter) =
            http.shared_object_mut::<DefaultLoginPageGeneratingFilter>()
        else {
            return;
        };
        if self.is_custom_login_page() {
            return;
        }
        login_page_generating_filter.set_one_time_token_enabled(true);
        login_page_generating_filter
            .set_one_time_token_generation_url(self.token_generating_url.clone());
        if login_page_generating_filter.get_login_page_url().is_none() {
            login_page_generating_filter
                .set_login_page_url(DefaultLoginPageGeneratingFilter::DEFAULT_LOGIN_PAGE_URL);
            login_page_generating_filter.set_failure_url(format!(
                "{}?{}",
                DefaultLoginPageGeneratingFilter::DEFAULT_LOGIN_PAGE_URL,
                DefaultLoginPageGeneratingFilter::ERROR_PARAMETER_NAME
            ));
            login_page_generating_filter.set_logout_success_url(format!(
                "{}?logout",
                DefaultLoginPageGeneratingFilter::DEFAULT_LOGIN_PAGE_URL
            ));
        }
    }

    /// Registers the `DefaultOneTimeTokenSubmitPageGeneratingFilter`.
    fn configure_submit_page(&mut self, http: &mut H) {
        if !self.submit_page_enabled {
            return;
        }
        let mut submit_page = DefaultOneTimeTokenSubmitPageGeneratingFilter::new();
        submit_page.set_resolve_hidden_inputs(Arc::new(resolve_hidden_inputs));
        submit_page.set_request_matcher(Arc::new(
            DEFAULT_BUILDER
                .get_or_init(Default::default)
                .matcher(Some(HttpMethod::GET), &self.default_submit_page_url),
        ));
        submit_page.set_login_processing_url(
            self.get_login_processing_url()
                .unwrap_or(OneTimeTokenAuthenticationFilter::DEFAULT_LOGIN_PROCESSING_URL),
        );
        http.add_filter(submit_page);
    }

    /// Registers the `GenerateOneTimeTokenFilter` and serves the default CSS.
    fn configure_ott_generate_filter(&mut self, http: &mut H) {
        let token_service = self.get_one_time_token_service();
        let success_handler = self.get_one_time_token_generation_success_handler();
        let mut generate_filter = GenerateOneTimeTokenFilter::new(token_service, success_handler);
        generate_filter.set_request_matcher(Arc::new(
            DEFAULT_BUILDER
                .get_or_init(Default::default)
                .matcher(Some(HttpMethod::POST), &self.token_generating_url),
        ));
        generate_filter.set_request_resolver(self.get_generate_request_resolver());
        http.add_filter(generate_filter);
        http.add_filter(DefaultResourcesFilter::css());
    }
}

/// Asserts that a configuration URL is set.
///
/// # Panics
///
/// Panics with `<name> cannot be null or empty`, the message the reference
/// implementation reports for an empty URL.
fn assert_has_text(value: &str, name: &str) {
    assert!(!value.trim().is_empty(), "{name} cannot be null or empty");
}

/// Resolves the hidden inputs (such as a CSRF token) that are rendered in the
/// default submit page form.
///
/// The CSRF filter exposes the token it loaded as a request attribute, so the
/// form can echo the token back on the submission that same filter validates. A
/// request that carries no token — CSRF protection disabled, or the filter not
/// part of the chain — renders no hidden input at all.
fn resolve_hidden_inputs(request: &dyn HttpRequest) -> HashMap<String, String> {
    let Some(AnyValue::BoxedValue(value)) =
        request.get_attribute(std::any::type_name::<&dyn CsrfToken>())
    else {
        return HashMap::new();
    };

    // `Box<dyn AnyClone>` satisfies the blanket `AnyClone` impl as well, so the
    // box has to be dereferenced before the erased value is reached.
    let Some(token) = (**value).as_any().downcast_ref::<Arc<dyn CsrfToken>>() else {
        return HashMap::new();
    };

    HashMap::from([(token.parameter_name().to_owned(), token.token().to_owned())])
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
    H: 'static,
{
    fn init(&mut self, http: &mut H) {
        // A login page given through the DSL replaces the generated one, so it is
        // installed before the login page integration runs.
        if let Some(login_page) = self.custom_login_page.clone() {
            <Self as BaseAuthenticationFilterConfigurerExt<H>>::login_page(
                self,
                login_page.as_ref(),
                http,
            );
        }

        if self.get_login_processing_url().is_none() {
            self.login_processing_url(
                OneTimeTokenAuthenticationFilter::DEFAULT_LOGIN_PROCESSING_URL,
            );
        }
        self.base.init(http);

        let authentication_provider = self.get_authentication_provider();
        http.authentication_provider(authentication_provider);

        self.init_default_login_filter(http);

        let entry_point = self.base.get_authentication_entry_point().cloned();
        let request_matcher = self.base.get_authentication_entry_point_matcher(http);
        if let Some(error_handling) = http.configurer_mut::<ErrorHandlingConfigurer<H>>() {
            error_handling.default_denied_handler_for_missing_authority_with_builder(
                |builder| {
                    if let Some(entry_point) = entry_point {
                        builder.add_entry_point_for(Arc::new(entry_point), request_matcher.clone());
                    }
                },
                FactorGrantedAuthority::OTT_AUTHORITY,
            );
        }
    }

    fn configure(&mut self, http: &mut H) {
        self.base.configure(http);
        self.configure_submit_page(http);
        self.configure_ott_generate_filter(http);
    }
}

impl<H> BaseAuthenticationFilterConfigurerExt<H> for OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn login_processing_url(&mut self, login_processing_url: &str) {
        self.base.login_processing_url(login_processing_url);
    }

    fn login_page(&mut self, login_page: &str, http: &mut H) {
        self.base.login_page(login_page, http);
    }

    fn create_login_processing_url_matcher(
        &self,
        login_processing_url: &str,
    ) -> Arc<dyn RequestMatcher> {
        Arc::new(
            DEFAULT_BUILDER
                .get_or_init(Default::default)
                .matcher(Some(HttpMethod::POST), login_processing_url),
        )
    }
}

impl<H> Default for OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn default() -> Self {
        Self {
            one_time_token_service: None,
            token_generating_url: DEFAULT_GENERATE_URL.to_string(),
            one_time_token_generation_success_handler: None,
            authentication_provider: None,
            user_details_service: None,
            request_resolver: None,
            default_submit_page_url: DEFAULT_SUBMIT_PAGE_URL.to_string(),
            submit_page_enabled: true,
            custom_login_page: None,
            base: BaseAuthenticationFilterConfigurer::new(
                OneTimeTokenAuthenticationFilter::default(),
                Some(OneTimeTokenAuthenticationFilter::DEFAULT_LOGIN_PROCESSING_URL),
            ),
        }
    }
}

impl<H> Deref for OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseAuthenticationFilterConfigurer<H, Self, OneTimeTokenAuthenticationFilter>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for OneTimeTokenLoginConfigurer<H>
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
    use std::sync::{Arc, Mutex, RwLock};

    use axum::{body::Body, extract::Request, response::Response};
    use next_web_core::{
        any::AnyValue,
        http::{
            filter::{FilterError, HttpFilter, HttpFilterChain},
            HttpMethod, HttpResponse,
        },
        ApplicationContext,
    };

    use super::*;
    use crate::{
        authentication::ott::{
            GenerateOneTimeTokenRequest, OneTimeToken, OneTimeTokenAuthenticationToken,
            OneTimeTokenService,
        },
        config::authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
        config::web::builders::{HttpSecurity, StandaloneApplicationContext},
        core::{userdetails::User, Authentication},
        provisioning::InMemoryUserDetailsManager,
        web::{
            csrf::{CsrfToken, DefaultCsrfToken},
            security_filter_chain::SecurityFilterChain,
        },
    };

    /// Returns a configurer whose application context contributes nothing.
    fn configurer() -> OneTimeTokenLoginConfigurer<HttpSecurity> {
        let context: Arc<RwLock<Box<dyn ApplicationContext>>> =
            Arc::new(RwLock::new(Box::new(StandaloneApplicationContext::default())
                as Box<dyn ApplicationContext>));

        OneTimeTokenLoginConfigurer::new(context)
    }

    /// Returns the user details service of a single user.
    fn user_details_service(
        username: &str,
    ) -> Arc<dyn crate::core::userdetails::UserDetailsService> {
        Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
            User::with_username(username)
                .password(Some("password".to_string()))
                .build(),
        )]))
    }

    /// Returns a configurer that can authenticate the given user.
    fn configurer_with_user(username: &str) -> OneTimeTokenLoginConfigurer<HttpSecurity> {
        let mut configurer = configurer();
        configurer.user_details_service(user_details_service(username));

        configurer
    }

    #[test]
    fn the_token_service_is_resolved_once() {
        let mut configurer = configurer();

        let first = configurer.get_one_time_token_service();
        let second = configurer.get_one_time_token_service();

        assert!(
            Arc::ptr_eq(&first, &second),
            "both call sites share one token store"
        );
    }

    #[tokio::test]
    async fn the_provider_redeems_a_token_generated_by_the_filter() {
        let mut configurer = configurer_with_user("Max");
        let issued = configurer
            .get_one_time_token_service()
            .generate(GenerateOneTimeTokenRequest::new("Max"));
        let authentication: Arc<dyn Authentication> =
            Arc::new(OneTimeTokenAuthenticationToken::new(issued.token_value()));

        let result = configurer
            .get_authentication_provider()
            .authenticate(&authentication)
            .await;

        assert!(
            matches!(result, Ok(Some(_))),
            "the provider finds the token the generate filter issued"
        );
    }

    #[test]
    fn the_request_resolver_is_resolved_once() {
        let mut configurer = configurer();

        let first = configurer.get_generate_request_resolver();
        let second = configurer.get_generate_request_resolver();

        assert!(Arc::ptr_eq(&first, &second), "the resolver is reused");
    }

    #[test]
    fn a_token_service_from_the_dsl_replaces_the_default() {
        let mut configurer = configurer();
        let custom: Arc<dyn OneTimeTokenService> = Arc::new(InMemoryOneTimeTokenService::new());
        configurer.token_service(custom.clone());

        assert!(Arc::ptr_eq(&configurer.get_one_time_token_service(), &custom));
    }

    #[test]
    #[should_panic(
        expected = "A OneTimeTokenGenerationSuccessHandler is required to enable oneTimeTokenLogin()."
    )]
    fn a_missing_success_handler_is_reported() {
        configurer().get_one_time_token_generation_success_handler();
    }

    #[test]
    #[should_panic(expected = "or pass it to the oneTimeTokenLogin() DSL")]
    fn a_missing_user_details_service_is_reported() {
        configurer().get_authentication_provider();
    }

    /// Returns a request with the given method and path.
    fn request(method: HttpMethod, path: &str) -> Request {
        Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .expect("a valid request")
    }

    #[test]
    fn the_csrf_token_is_rendered_as_a_hidden_input() {
        let mut request = request(HttpMethod::GET, "/login/ott");
        let token: Arc<dyn CsrfToken> =
            Arc::new(DefaultCsrfToken::new("X-CSRF-TOKEN", "_csrf", "csrf-token-value"));
        request.set_attribute(
            std::any::type_name::<&dyn CsrfToken>(),
            AnyValue::BoxedValue(Box::new(token)),
        );

        assert_eq!(
            resolve_hidden_inputs(&request),
            HashMap::from([("_csrf".to_owned(), "csrf-token-value".to_owned())])
        );
    }

    #[test]
    fn no_hidden_input_is_rendered_without_a_csrf_token() {
        assert!(resolve_hidden_inputs(&request(HttpMethod::GET, "/login/ott")).is_empty());
    }

    /// Returns an HTTP security a configurer can be initialized against.
    fn http_security() -> HttpSecurity {
        HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new())
    }

    #[test]
    #[should_panic(expected = "tokenGeneratingUrl cannot be null or empty")]
    fn an_empty_token_generating_url_is_rejected() {
        configurer().token_generating_url("  ");
    }

    #[test]
    #[should_panic(expected = "loginProcessingUrl cannot be null or empty")]
    fn an_empty_login_processing_url_is_rejected() {
        configurer().login_processing_url("");
    }

    #[test]
    #[should_panic(expected = "submitPageUrl cannot be null or empty")]
    fn an_empty_submit_page_url_is_rejected() {
        configurer().default_submit_page_url("");
    }

    #[test]
    fn a_custom_login_page_becomes_the_entry_point() {
        let mut http = http_security();
        let mut configurer = configurer_with_user("user");
        configurer.login_page("/custom-login");

        configurer.init(&mut http);

        assert!(configurer.is_custom_login_page());
        assert_eq!(
            configurer
                .get_authentication_entry_point()
                .map(|point| point.get_login_form_url().to_owned()),
            Some("/custom-login".to_owned())
        );
    }

    #[test]
    fn init_enables_the_one_time_token_form_on_the_login_page() {
        let mut http = http_security();
        http.set_shared_object(DefaultLoginPageGeneratingFilter::default());

        configurer_with_user("user").init(&mut http);

        let login_page = http
            .shared_object::<DefaultLoginPageGeneratingFilter>()
            .expect("the login page is shared");
        assert!(login_page.is_one_time_token_enabled());
        assert_eq!(
            login_page.get_one_time_token_generation_url(),
            Some("/ott/generate")
        );
    }

    #[test]
    fn init_leaves_the_login_page_alone_when_a_custom_login_page_is_set() {
        let mut http = http_security();
        http.set_shared_object(DefaultLoginPageGeneratingFilter::default());
        let mut configurer = configurer_with_user("user");
        configurer.login_page("/custom-login");

        configurer.init(&mut http);

        let login_page = http
            .shared_object::<DefaultLoginPageGeneratingFilter>()
            .expect("the login page is shared");
        assert!(
            !login_page.is_one_time_token_enabled(),
            "a custom login page carries no generated form"
        );
    }

    /// The success handler that records the tokens the generate filter issues.
    #[derive(Clone)]
    struct RecordingHandler(Arc<Mutex<Vec<String>>>);

    impl OneTimeTokenGenerationSuccessHandler for RecordingHandler {
        fn handle(
            &self,
            _request: &mut dyn HttpRequest,
            response: &mut dyn HttpResponse,
            ott: &dyn OneTimeToken,
        ) {
            self.0
                .lock()
                .expect("the issued tokens are not poisoned")
                .push(ott.token_value().to_owned());
            response.set_redirect("/login/ott");
        }
    }

    /// Runs the filters of a chain in order, the way the chain proxy does.
    #[derive(Clone)]
    struct Chain {
        filters: Arc<Vec<Arc<dyn HttpFilter>>>,
        index: usize,
    }

    impl Chain {
        /// Returns a chain that starts at the first filter.
        fn new(filters: Arc<Vec<Arc<dyn HttpFilter>>>) -> Self {
            Self { filters, index: 0 }
        }

        /// Runs the filter at the current index and everything after it.
        async fn run(
            &self,
            request: &mut dyn HttpRequest,
            response: &mut dyn HttpResponse,
        ) -> Result<(), FilterError> {
            HttpFilterChain::do_filter(self, request, response).await
        }
    }

    #[next_web_core::async_trait]
    impl HttpFilterChain for Chain {
        async fn do_filter(
            &self,
            request: &mut dyn HttpRequest,
            response: &mut dyn HttpResponse,
        ) -> Result<(), FilterError> {
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

    /// Returns the `Location` header of a response.
    fn redirect_of(response: &Response) -> Option<String> {
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }

    #[tokio::test]
    async fn a_generated_token_authenticates_a_login_request() {
        let mut http = http_security();
        let issued = Arc::new(Mutex::new(Vec::<String>::new()));
        let mut configurer = configurer_with_user("user");
        configurer.token_generation_success_handler(Arc::new(RecordingHandler(issued.clone())));
        http.with(configurer, |_| {});

        let chain = http.build();
        let filters = Arc::new(chain.get_filters().to_vec());

        // A generated token reaches the success handler.
        let mut generate = request(HttpMethod::POST, "/ott/generate?username=user");
        let mut generated_response = Response::default();
        Chain::new(filters.clone())
            .run(&mut generate, &mut generated_response)
            .await
            .expect("the chain runs");
        assert!(
            generated_response.status().is_redirection(),
            "the success handler redirects, status was {}",
            generated_response.status()
        );
        assert_eq!(
            redirect_of(&generated_response),
            Some("/login/ott".to_owned())
        );
        let token = issued
            .lock()
            .expect("the issued tokens are not poisoned")
            .last()
            .cloned()
            .expect("a token was issued");

        // Redeeming the token authenticates the user.
        let mut login = request(
            HttpMethod::POST,
            &format!("/login/ott?token={token}"),
        );
        let mut login_response = Response::default();
        Chain::new(filters.clone())
            .run(&mut login, &mut login_response)
            .await
            .expect("the chain runs");
        assert!(
            login_response.status().is_redirection(),
            "the login redirects, status was {}",
            login_response.status()
        );
        assert_eq!(redirect_of(&login_response), Some("/".to_owned()));

        // The same token cannot be redeemed a second time.
        let mut replay = request(
            HttpMethod::POST,
            &format!("/login/ott?token={token}"),
        );
        let mut replay_response = Response::default();
        Chain::new(filters)
            .run(&mut replay, &mut replay_response)
            .await
            .expect("the chain runs");
        assert_eq!(
            redirect_of(&replay_response),
            Some("/login?error".to_owned()),
            "a consumed token is rejected"
        );
    }
}
