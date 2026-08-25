use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    async_trait,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        required::Required,
    },
    util::StringUtils,
    ApplicationContext,
};

use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        web::{
            configurers::{
                BaseHttpConfigurer, ErrorHandlingConfigurer, LogoutConfigurer,
                SessionManagementConfigurer,
            },
            http_security_builder::HttpSecurityBuilder,
            BaseRequestMatcherRegistry,
        },
    },
    web::{
        access::{AccessDeniedHandler, AccessDeniedHandlerImpl, DelegatingAccessDeniedHandler},
        authentication::session::SessionAuthenticationStrategy,
        csrf::{
            CookieCsrfTokenRepository, CsrfAuthenticationStrategy, CsrfFilter, CsrfLogoutHandler,
            CsrfToken, CsrfTokenRepository, CsrfTokenRequestAttributeHandler,
            CsrfTokenRequestHandler, CsrfTokenRequestResolver, DeferredCsrfToken,
            HttpSessionCsrfTokenRepository,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
        session::{InvalidSessionAccessDeniedHandler, InvalidSessionStrategy},
        util::matcher::{
            AndRequestMatcher, NegatedRequestMatcher, OrRequestMatcher, RequestMatcher,
        },
    },
};

#[derive(Clone)]
pub struct CsrfConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    csrf_token_repository: Arc<dyn CsrfTokenRepository>,
    require_csrf_protection_matcher: Arc<dyn RequestMatcher>,
    ignored_csrf_protection_matchers: Vec<Arc<dyn RequestMatcher>>,
    session_authentication_strategy: Option<Arc<dyn SessionAuthenticationStrategy>>,
    request_handler: Option<Arc<dyn CsrfTokenRequestHandler>>,

    inner: BaseHttpConfigurer<Self, H>,
}

impl<H> CsrfConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    pub fn new(_ctx: &ApplicationContext) -> Self {
        Self {
            csrf_token_repository: Arc::new(HttpSessionCsrfTokenRepository::default()),
            require_csrf_protection_matcher: CsrfFilter::default_csrf_matcher(),
            ignored_csrf_protection_matchers: Default::default(),
            session_authentication_strategy: Default::default(),
            request_handler: Default::default(),

            inner: Default::default(),
        }
    }

    /// Specify the CsrfTokenRepository to use. The default is an HttpSessionCsrfTokenRepository.
    pub fn csrf_token_repository<T>(mut self, csrf_token_repository: T) -> Self
    where
        T: CsrfTokenRepository,
        T: 'static,
    {
        self.csrf_token_repository = Arc::new(csrf_token_repository);
        self
    }

    /// Specify the RequestMatcher to use for determining when CSRF should be applied. The default is
    /// to ignore GET, HEAD, TRACE, OPTIONS and process all other requests.
    pub fn require_csrf_protection_matcher<T>(mut self, require_csrf_protection_matcher: T) -> Self
    where
        T: RequestMatcher,
        T: 'static,
    {
        self.require_csrf_protection_matcher = Arc::new(require_csrf_protection_matcher);
        self
    }

    /// Specify a CsrfTokenRequestHandler to use for making the CsrfToken available as a request attribute.
    pub fn csrf_token_request_handler<T>(mut self, request_handler: T) -> Self
    where
        T: CsrfTokenRequestHandler,
        T: 'static,
    {
        self.request_handler = Some(Arc::new(request_handler));
        self
    }

    /// Allows specifying HttpRequests that should not use CSRF Protection even if they match the require_csrf_protection_matcher(request_matcher).
    pub fn ignored_csrf_protection_matchers(
        mut self,
        request_matchers: Vec<Arc<dyn RequestMatcher>>,
    ) -> Self {
        self.ignored_csrf_protection_matchers
            .extend(request_matchers);

        self
    }

    /// Allows specifying HttpRequest that should not use CSRF Protection even if they match the require_csrf_protection_matcher(request_matcher).
    pub fn ignored_csrf_protection_matchers_with_string(
        mut self,
        patterns: Vec<&'static str>,
    ) -> Self {
        let request_matchers = BaseRequestMatcherRegistry::<()>::default()
            .request_matchers(patterns)
            .take_request_matchers();
        self.ignored_csrf_protection_matchers
            .extend(request_matchers);

        self
    }

    /// Specify the SessionAuthenticationStrategy to use. The default is a CsrfAuthenticationStrategy.
    pub fn session_authentication_strategy<T>(mut self, session_authentication_strategy: T) -> Self
    where
        T: SessionAuthenticationStrategy,
        T: 'static,
    {
        self.session_authentication_strategy = Some(Arc::new(session_authentication_strategy));
        self
    }

    /// Sensible CSRF defaults when used in combination with a single page application.
    /// Creates a cookie-based token repository and a custom request handler to resolve the actual token value instead of
    /// the encoded token.
    pub fn spa(&mut self) -> &mut Self {
        self.csrf_token_repository = Arc::new(CookieCsrfTokenRepository::with_http_only_false());
        self.request_handler = Some(Arc::new(SpaCsrfTokenRequestHandler::default()));

        self
    }
}

impl<H> CsrfConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    /// Gets the final RequestMatcher to use by combining the require_csrf_protection_matcher(request_matcher) and any ignore().
    fn get_require_csrf_protection_matcher(&self) -> Arc<dyn RequestMatcher> {
        if self.ignored_csrf_protection_matchers.is_empty() {
            return self.require_csrf_protection_matcher.to_owned();
        }

        let negated_request_matcher = NegatedRequestMatcher::new(OrRequestMatcher::new(
            self.ignored_csrf_protection_matchers.to_owned(),
        ));
        let request_matchers = vec![
            self.require_csrf_protection_matcher.to_owned(),
            Arc::new(negated_request_matcher),
        ];

        return Arc::new(AndRequestMatcher::new(request_matchers));
    }

    /// Gets the default AccessDeniedHandler from the ErrorHandlingConfigurer::get_access_denied_handler(HttpSecurityBuilder) or create a AccessDeniedHandlerImpl if not available.
    fn get_default_access_denied_handler(&self, http: &mut H) -> Arc<dyn AccessDeniedHandler> {
        http.configurer::<ErrorHandlingConfigurer<H>>()
            .and_then(|error_config| error_config.get_access_denied_handler().map(Clone::clone))
            .unwrap_or(Arc::new(AccessDeniedHandlerImpl::default()))
    }

    /// Gets the default InvalidSessionStrategy from the SessionManagementConfigurer::get_invalid_session_strategy() or Option::None if not available.
    fn get_invalid_session_strategy(
        &self,
        http: &mut H,
    ) -> Option<Arc<dyn InvalidSessionStrategy>> {
        http.configurer_mut::<SessionManagementConfigurer<H>>()?
            .get_invalid_session_strategy()
    }

    /// Creates the AccessDeniedHandler from the result of get_default_access_denied_handler(HttpSecurityBuilder) and get_invalid_session_strategy(HttpSecurityBuilder).
    /// If get_invalid_session_strategy(HttpSecurityBuilder) is non-null, then a DelegatingAccessDeniedHandler is used in combination with InvalidSessionAccessDeniedHandler
    /// and the get_default_access_denied_handler(HttpSecurityBuilder). Otherwise, only get_default_access_denied_handler(HttpSecurityBuilder) is used.
    fn create_access_denied_handler(&self, http: &mut H) -> Arc<dyn AccessDeniedHandler> {
        let invalid_session_strategy = self.get_invalid_session_strategy(http);
        let default_access_denied_handler = self.get_default_access_denied_handler(http);

        let Some(invalid_session_strategy) = invalid_session_strategy else {
            return default_access_denied_handler;
        };

        let invalid_session_denied_handler =
            InvalidSessionAccessDeniedHandler::new(invalid_session_strategy);

        let mut handlers = BTreeMap::new();
        handlers.insert(
            "MissingCsrfToken",
            Arc::new(invalid_session_denied_handler) as Arc<dyn AccessDeniedHandler>,
        );

        Arc::new(DelegatingAccessDeniedHandler::new(
            handlers,
            default_access_denied_handler,
        ))
    }

    fn get_session_authentication_strategy(&self) -> Arc<dyn SessionAuthenticationStrategy> {
        match self.session_authentication_strategy.as_ref() {
            Some(session_authentication_strategy) => session_authentication_strategy.clone(),
            None => {
                let mut csrf_authentication_strategy =
                    CsrfAuthenticationStrategy::new(self.csrf_token_repository.clone());
                if let Some(request_handler) = self.request_handler.as_ref() {
                    csrf_authentication_strategy.set_request_handler(request_handler.clone());
                }

                return Arc::new(csrf_authentication_strategy);
            }
        }
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for CsrfConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, http: &mut H) {
        let mut filter = CsrfFilter::new(self.csrf_token_repository.to_owned());
        let require_csrf_protection_matcher = self.get_require_csrf_protection_matcher();
        filter.set_require_csrf_protection_matcher(require_csrf_protection_matcher);

        let access_denied_handler = self.create_access_denied_handler(http);

        // TODO
        // let registry = self.get_observation_registry();
        // let observable =  ObservationMarkingAccessDeniedHandler::new(registry);
        // access_denied_handler =  CompositeAccessDeniedHandler::new(observable, access_denied_handler.clone());

        filter.set_access_denied_handler(access_denied_handler);

        let logout_configurer = http.configurer_mut::<LogoutConfigurer<H>>();
        if let Some(logout_configurer) = logout_configurer {
            logout_configurer.add_logout_handler(Arc::new(CsrfLogoutHandler::new(
                self.csrf_token_repository.clone(),
            )));
        }

        let session_configurer = http.configurer_mut::<SessionManagementConfigurer<H>>();
        if let Some(session_configurer) = session_configurer {
            session_configurer
                .add_session_authentication_strategy(self.get_session_authentication_strategy());
        }

        if let Some(request_handler) = self.request_handler.take() {
            filter.set_request_handler(request_handler);
        }
        self.inner.get_mut_object().post_process(&mut filter);

        http.add_filter(filter);
    }
}

impl<H> Deref for CsrfConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for CsrfConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

struct SpaCsrfTokenRequestHandler {
    plain: CsrfTokenRequestAttributeHandler,
    xor: CsrfTokenRequestAttributeHandler,
}

#[async_trait]
impl CsrfTokenRequestHandler for SpaCsrfTokenRequestHandler {
    async fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        csrf_token: &mut dyn DeferredCsrfToken,
    ) {
        self.xor.handle(request, response, csrf_token);
    }
}

impl CsrfTokenRequestResolver for SpaCsrfTokenRequestHandler {
    fn resolve_csrf_token_value(
        &self,
        request: &mut dyn HttpRequest,
        csrf_token: &dyn CsrfToken,
    ) -> Option<String> {
        let header_value = request.header(csrf_token.header_name());

        if header_value.map(StringUtils::has_text).unwrap_or_default() {
            self.plain.resolve_csrf_token_value(request, csrf_token)
        } else {
            self.xor.resolve_csrf_token_value(request, csrf_token)
        }
    }
}

impl Default for SpaCsrfTokenRequestHandler {
    fn default() -> Self {
        let mut xor = CsrfTokenRequestAttributeHandler::default();
        xor.set_csrf_request_attribute_name(None);

        Self {
            plain: Default::default(),
            xor,
        }
    }
}
