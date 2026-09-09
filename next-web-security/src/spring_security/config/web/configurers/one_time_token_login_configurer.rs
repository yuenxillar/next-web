use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    http::HttpMethod, traits::http::http_request::HttpRequest, ApplicationContext,
};

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
    context: ApplicationContext,

    one_time_token_service: Option<Arc<dyn OneTimeTokenService>>,
    token_generating_url: String,
    one_time_token_generation_success_handler:
        Option<Arc<dyn OneTimeTokenGenerationSuccessHandler>>,
    authentication_provider: Option<Arc<dyn AuthenticationProvider>>,
    request_resolver: Option<Arc<dyn GenerateOneTimeTokenRequestResolver>>,

    default_submit_page_url: String,
    submit_page_enabled: bool,

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
    /// * `context` - the `ApplicationContext` used to resolve beans such as the
    ///   `OneTimeTokenService`, `UserDetailsService` and
    ///   `OneTimeTokenGenerationSuccessHandler`.
    pub fn new(context: &ApplicationContext) -> Self {
        let mut configurer = Self::default();
        configurer.context = context.clone();
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

    /// Specifies the URL that a one-time token generation request will be processed
    /// on. Defaults to `/ott/generate`.
    pub fn token_generating_url(&mut self, token_generating_url: &str) -> &mut Self {
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
    pub fn default_submit_page_url(&mut self, submit_page_url: &str) -> &mut Self {
        self.default_submit_page_url = submit_page_url.to_string();
        self.show_default_submit_page(true);
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

    /// Resolves the `OneTimeTokenService` to use. Falls back to a bean from the
    /// `ApplicationContext`, and finally to an `InMemoryOneTimeTokenService`.
    fn get_one_time_token_service(&self) -> Arc<dyn OneTimeTokenService> {
        if let Some(service) = &self.one_time_token_service {
            return service.clone();
        }
        self.context
            .get_single_option::<Arc<dyn OneTimeTokenService>>()
            .map(Clone::clone)
            .unwrap_or_else(|| Arc::new(InMemoryOneTimeTokenService::new()))
    }

    /// Resolves the `OneTimeTokenGenerationSuccessHandler` to use. Falls back to a
    /// bean from the `ApplicationContext`. A handler is required, so this method
    /// panics if none can be resolved.
    fn get_one_time_token_generation_success_handler(
        &self,
    ) -> Arc<dyn OneTimeTokenGenerationSuccessHandler> {
        if let Some(handler) = &self.one_time_token_generation_success_handler {
            return handler.clone();
        }
        self.context
            .get_single_option::<Arc<dyn OneTimeTokenGenerationSuccessHandler>>()
            .map(Clone::clone)
            .expect(
                "A OneTimeTokenGenerationSuccessHandler is required to enable oneTimeTokenLogin(). \
                 Please provide it as a bean or pass it to the oneTimeTokenLogin() DSL.",
            )
    }

    /// Resolves the `GenerateOneTimeTokenRequestResolver` to use. Falls back to a
    /// bean from the `ApplicationContext`, and finally to a
    /// `DefaultGenerateOneTimeTokenRequestResolver`.
    fn get_generate_request_resolver(&self) -> Arc<dyn GenerateOneTimeTokenRequestResolver> {
        if let Some(resolver) = &self.request_resolver {
            return resolver.clone();
        }
        self.context
            .get_single_option::<Arc<dyn GenerateOneTimeTokenRequestResolver>>()
            .map(Clone::clone)
            .unwrap_or_else(|| Arc::new(DefaultGenerateOneTimeTokenRequestResolver::default()))
    }

    /// Builds the `AuthenticationProvider` used to authenticate one-time tokens.
    fn get_authentication_provider(&self) -> Arc<dyn AuthenticationProvider> {
        if let Some(provider) = &self.authentication_provider {
            return provider.clone();
        }
        let user_details_service = self
            .context
            .get_single_option::<Arc<dyn crate::core::userdetails::UserDetailsService>>()
            .map(Clone::clone)
            .expect(
                "A UserDetailsService is required to enable oneTimeTokenLogin(). \
                 Please provide it as a bean.",
            );
        Arc::new(OneTimeTokenAuthenticationProvider::new(
            self.get_one_time_token_service(),
            user_details_service,
        ))
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
        let mut generate_filter = GenerateOneTimeTokenFilter::new(
            self.get_one_time_token_service(),
            self.get_one_time_token_generation_success_handler(),
        );
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

/// Resolves the hidden inputs (such as a CSRF token) that should be rendered in
/// the default submit page form. When CSRF protection is enabled the token is
/// expected to be supplied through the request; the current implementation
/// returns an empty map as no CSRF token type is exposed through the request
/// attribute API yet.
fn resolve_hidden_inputs(_request: &dyn HttpRequest) -> HashMap<String, String> {
    HashMap::new()
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for OneTimeTokenLoginConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
    H: 'static,
{
    fn init(&mut self, http: &mut H) {
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
            context: ApplicationContext::default(),
            one_time_token_service: None,
            token_generating_url: DEFAULT_GENERATE_URL.to_string(),
            one_time_token_generation_success_handler: None,
            authentication_provider: None,
            request_resolver: None,
            default_submit_page_url: DEFAULT_SUBMIT_PAGE_URL.to_string(),
            submit_page_enabled: true,
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
