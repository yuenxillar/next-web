use std::{collections::BTreeMap, sync::Arc};

use next_web_core::{
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
                base_http_configurer::BaseHttpConfigurer, logout_configurer::LogoutConfigurer,
                ErrorHandlingConfigurer, SessionManagementConfigurer,
            },
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::{
        access::{AccessDeniedHandler, AccessDeniedHandlerImpl, DelegatingAccessDeniedHandler},
        authentication::session::SessionAuthenticationStrategy,
        csrf::{
            CookieCsrfTokenRepository, CsrfAuthenticationStrategy, CsrfFilter, CsrfLogoutHandler,
            CsrfToken, CsrfTokenRepository, CsrfTokenRequestAttributeHandler,
            CsrfTokenRequestHandler, CsrfTokenRequestResolver, HttpSessionCsrfTokenRepository,
            MissingCsrfTokenError,
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

    base_http_configurer: BaseHttpConfigurer<Self, H>,
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
            base_http_configurer: Default::default(),
        }
    }

    pub fn csrf_token_repository<T>(mut self, csrf_token_repository: T) -> Self
    where
        T: CsrfTokenRepository,
        T: 'static,
    {
        self.csrf_token_repository = Arc::new(csrf_token_repository);
        self
    }

    pub fn require_csrf_protection_matcher<T>(mut self, require_csrf_protection_matcher: T) -> Self
    where
        T: RequestMatcher,
        T: 'static,
    {
        self.require_csrf_protection_matcher = Arc::new(require_csrf_protection_matcher);
        self
    }

    pub fn csrf_token_request_handler<T>(mut self, request_handler: T) -> Self
    where
        T: CsrfTokenRequestHandler,
        T: 'static,
    {
        self.request_handler = Some(Arc::new(request_handler));
        self
    }

    pub fn ignored_csrf_protection_matchers(
        mut self,
        ignored_csrf_protection_matchers: Vec<Arc<dyn RequestMatcher>>,
    ) -> Self {
        self.ignored_csrf_protection_matchers
            .extend(ignored_csrf_protection_matchers);

        self
    }

    pub fn ignored_csrf_protection_matchers_with_string<T, const N: usize>(
        mut self,
        ignored_csrf_protection_matchers: [T; N],
    ) -> Self
    where
        T: Into<String>,
    {
        let a = ignored_csrf_protection_matchers
            .into_iter()
            .map(|s| s.into())
            .collect::<Vec<_>>();

        todo!();
    }

    pub fn session_authentication_strategy<T>(mut self, session_authentication_strategy: T) -> Self
    where
        T: SessionAuthenticationStrategy,
        T: 'static,
    {
        self.session_authentication_strategy = Some(Arc::new(session_authentication_strategy));
        self
    }

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
    fn get_require_csrf_protection_matcher(&self) -> Arc<dyn RequestMatcher> {
        if self.ignored_csrf_protection_matchers.is_empty() {
            return self.require_csrf_protection_matcher.clone();
        }

        let negated_request_matcher = NegatedRequestMatcher::new(OrRequestMatcher::new(
            self.ignored_csrf_protection_matchers.clone(),
        ));
        let request_matchers = vec![
            self.require_csrf_protection_matcher.clone(),
            Arc::new(negated_request_matcher),
        ];

        return Arc::new(AndRequestMatcher::new(request_matchers));
    }

    fn get_default_access_denied_handler(&self, http: &mut H) -> Arc<dyn AccessDeniedHandler> {
        http.get_configurer::<ErrorHandlingConfigurer<H>>()
            .map(|exception_config| exception_config.get_access_denied_handler())
            .unwrap_or_default()
            .unwrap_or(Arc::new(AccessDeniedHandlerImpl::default()))
    }

    fn get_invalid_session_strategy(
        &self,
        http: &mut H,
    ) -> Option<Arc<dyn InvalidSessionStrategy>> {
        match http.get_configurer::<SessionManagementConfigurer<H>>() {
            Some(session_management) => session_management.get_invalid_session_strategy(),
            None => return None,
        }
    }

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
            std::any::type_name::<MissingCsrfTokenError>(),
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
                    csrf_authentication_strategy.set_request_handler(request_handler);
                }

                return Arc::new(csrf_authentication_strategy);
            }
        }
    }

    // fn get_observation_registry(
    //     &self
    // ) -> Arc<dyn ObservationRegistry> {}
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for CsrfConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, http: &mut H) {
        let mut filter = CsrfFilter::new(self.csrf_token_repository.clone());
        let require_csrf_protection_matcher = self.get_require_csrf_protection_matcher();
        filter.set_require_csrf_protection_matcher(require_csrf_protection_matcher);

        let access_denied_handler = self.create_access_denied_handler(http);

        // TODO
        // let registry = self.get_observation_registry();
        // let observable =  ObservationMarkingAccessDeniedHandler::new(registry);
        // access_denied_handler =  CompositeAccessDeniedHandler::new(observable, access_denied_handler.clone());

        filter.set_access_denied_handler(access_denied_handler);

        let logout_configurer = http.get_configurer::<LogoutConfigurer<H>>();
        if let Some(mut logout_configurer) = logout_configurer {
            logout_configurer
                .add_logout_handler(CsrfLogoutHandler::new(self.csrf_token_repository.clone()));
        }

        let session_configurer = http.get_configurer::<SessionManagementConfigurer<H>>();
        if let Some(session_configurer) = session_configurer {
            session_configurer
                .add_session_authentication_strategy(self.get_session_authentication_strategy());
        }

        if let Some(request_handler) = self.request_handler.take() {
            filter.set_request_handler(request_handler);
        }

        self.base_http_configurer
            .get_mut_object()
            .post_process(&mut filter);

        http.add_filter(filter);
    }
}

impl<H> Required<BaseHttpConfigurer<CsrfConfigurer<H>, H>> for CsrfConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<CsrfConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<CsrfConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

struct SpaCsrfTokenRequestHandler {
    plain: CsrfTokenRequestAttributeHandler,
    xor: CsrfTokenRequestAttributeHandler,
}

impl CsrfTokenRequestHandler for SpaCsrfTokenRequestHandler {
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        csrf_token: &dyn Fn() -> Arc<dyn CsrfToken>,
    ) {
        self.xor.handle(request, response, csrf_token);
    }
}

impl CsrfTokenRequestResolver for SpaCsrfTokenRequestHandler {
    fn resolve_csrf_token_value(
        &self,
        request: &mut dyn HttpRequest,
        csrf_token: &dyn crate::web::csrf::CsrfToken,
    ) -> Option<String> {
        let header_value = request.header(csrf_token.get_header_name());

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
