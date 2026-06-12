use std::{
    any::TypeId,
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::{atomic::Ordering, Arc},
};

use next_web_core::{
    async_trait,
    error::BoxError,
    traits::{
        any_clone::AnyClone,
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
        ordered::Ordered,
    },
    ApplicationContext,
};

use crate::{
    authentication::authentication_provider::AuthenticationProvider,
    authorization::AuthenticationManager,
    config::{
        authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
        base_configured_security_builder::{
            BaseConfiguredSecurityBuilder, BaseConfiguredSecurityBuilderExt, BuildState,
        },
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        web::{
            base_request_matcher_registry::BaseRequestMatcherRegistry,
            builders::filter_order_registration::FilterOrderRegistration,
            configurers::{
                oauth2::client::OidcLogoutConfigurer, AnonymousConfigurer,
                AuthorizationManagerRequestMatcherRegistry, AuthorizeHttpRequestsConfigurer,
                CorsConfigurer, CsrfConfigurer, ErrorHandlingConfigurer, FormLoginConfigurer,
                HeadersConfigurer, HttpBasicConfigurer, HttpsRedirectConfigurer, LogoutConfigurer,
                OAuth2AuthorizationServerConfigurer, OAuth2ClientConfigurer, OAuth2LoginConfigurer,
                OAuth2ResourceServerConfigurer, OneTimeTokenLoginConfigurer,
                PasswordManagementConfigurer, PortMapperConfigurer, RememberMeConfigurer,
                RequestCacheConfigurer, Saml2LoginConfigurer, Saml2LogoutConfigurer,
                Saml2MetadataConfigurer, SecurityContextConfigurer, SessionManagementConfigurer,
                WebAuthnConfigurer,
            },
            http_security_builder::HttpSecurityBuilder,
        },
    },
    core::userdetails::user_details_service::UserDetailsService,
    web::{
        default_security_filter_chain::DefaultSecurityFilterChain,
        util::matcher::{AnyRequestMatcher, Builder, OrRequestMatcher, RequestMatcher},
    },
};

/// Configures the security settings for an HTTP request.
pub struct HttpSecurity {
    request_matcher_configurer: RequestMatcherConfigurer,
    filters: Vec<OrderedFilter>,
    request_matcher: Arc<dyn RequestMatcher>,
    filter_orders: FilterOrderRegistration,
    authentication_manager: Option<Arc<dyn AuthenticationManager>>,

    base: BaseConfiguredSecurityBuilder<DefaultSecurityFilterChain, Self>,
}

impl HttpSecurity {
    pub fn new(
        authentication_builder: AuthenticationManagerBuilder,
        shared_objects: HashMap<TypeId, Box<dyn AnyClone>>,
    ) -> Self {
        let mut http = Self {
            filter_orders: Default::default(),
            request_matcher: Arc::new(AnyRequestMatcher),
            filters: Vec::new(),
            request_matcher_configurer: RequestMatcherConfigurer::new(),
            authentication_manager: None,

            base: Default::default(),
        };

        http.base.set_shared_object(authentication_builder);
        http.base.shared_objects.extend(shared_objects.into_iter());

        http
    }

    fn get_context(&self) -> &ApplicationContext {
        self.get_shared_object::<ApplicationContext>().unwrap()
    }

    // ------------------------------------------------------------------
    // getOrApply — the central configurer management pattern from Java.
    // Every configurer DSL method delegates to this single function.
    // ------------------------------------------------------------------

    /// If a configurer of type `C` is already registered, clone and return it.
    /// Otherwise, register the given configurer and return it.
    fn get_or_apply<C, F>(&mut self, configurer: C, mut f: F)
    where
        C: AnyClone + 'static,
        C: SecurityConfigurer<DefaultSecurityFilterChain, Self>,
        F: FnMut(&mut C),
    {
        if let Some(existing_config) = self.get_configurer::<C>() {
            f(existing_config);
        } else {
            self.with(configurer, f);
        }
    }

    // Filter helpers
    fn add_filter_at_offset<F, F1>(&mut self, filter: F, offset: i32)
    where
        F: HttpFilter,
        F1: HttpFilter,
    {
        let name = std::any::type_name::<F1>();
        let registered_filter_order = self
            .filter_orders
            .get_order_by_name(name)
            .unwrap_or_else(|| panic!("Filter {} has no registered order", name));
        let order = registered_filter_order + offset;
        self.filters
            .push(OrderedFilter::new(Arc::new(filter), order));
        self.filter_orders.put::<F>(order);
    }
}

impl HttpSecurity {
    pub fn headers<F>(mut self, headers: F) -> Self
    where
        F: FnMut(&mut HeadersConfigurer<Self>),
    {
        self.get_or_apply(HeadersConfigurer::default(), headers);

        self
    }

    pub fn cors<F>(mut self, cors: F) -> Self
    where
        F: FnMut(&mut CorsConfigurer<Self>),
    {
        self.get_or_apply(CorsConfigurer::default(), cors);

        self
    }

    pub fn session_management<F>(mut self, session_management: F) -> Self
    where
        F: FnMut(&mut SessionManagementConfigurer<Self>),
    {
        self.get_or_apply(SessionManagementConfigurer::default(), session_management);

        self
    }

    pub fn port_mapper<F>(mut self, port_mapper: F) -> Self
    where
        F: FnMut(&mut PortMapperConfigurer<Self>),
    {
        self.get_or_apply(PortMapperConfigurer::default(), port_mapper);

        self
    }

    pub fn remember_me<F>(mut self, remember_me: F) -> Self
    where
        F: FnMut(&mut RememberMeConfigurer<Self>),
    {
        self.get_or_apply(RememberMeConfigurer::default(), remember_me);

        self
    }

    pub fn authorize_http_requests<F>(mut self, mut authorize_http_requests: F) -> Self
    where
        F: FnMut(&mut AuthorizationManagerRequestMatcherRegistry),
    {
        let configurer = AuthorizeHttpRequestsConfigurer::<Self>::new(self.get_context());
        self.get_or_apply(configurer, |configurer| {
            authorize_http_requests(configurer.registry_mut());
        });

        self
    }

    pub fn request_cache<F>(mut self, request_cache: F) -> Self
    where
        F: FnMut(&mut RequestCacheConfigurer<Self>),
    {
        self.get_or_apply(RequestCacheConfigurer::default(), request_cache);

        self
    }

    pub fn error_handling<F>(mut self, error_handling: F) -> Self
    where
        F: FnMut(&mut ErrorHandlingConfigurer<Self>),
    {
        self.get_or_apply(ErrorHandlingConfigurer::default(), error_handling);

        self
    }

    pub fn security_context<F>(mut self, security_context: F) -> Self
    where
        F: FnMut(&mut SecurityContextConfigurer<Self>),
    {
        self.get_or_apply(SecurityContextConfigurer::default(), security_context);

        self
    }

    pub fn csrf<F>(mut self, csrf: F) -> Self
    where
        F: FnMut(&mut CsrfConfigurer<Self>),
    {
        self.get_or_apply(CsrfConfigurer::new(self.get_context()), csrf);

        self
    }

    pub fn logout<F>(mut self, logout: F) -> Self
    where
        F: FnMut(&mut LogoutConfigurer<Self>),
    {
        self.get_or_apply(LogoutConfigurer::default(), logout);

        self
    }

    pub fn anonymous<F>(mut self, anonymous: F) -> Self
    where
        F: FnMut(&mut AnonymousConfigurer<Self>),
    {
        self.get_or_apply(AnonymousConfigurer::default(), anonymous);

        self
    }

    pub fn form_login<F>(mut self, form_login: F) -> Self
    where
        F: FnMut(&mut FormLoginConfigurer<Self>),
    {
        self.get_or_apply(FormLoginConfigurer::default(), form_login);

        self
    }

    pub fn saml2_login<F>(mut self, saml2_login: F) -> Self
    where
        F: FnMut(&mut Saml2LoginConfigurer<Self>),
    {
        self.get_or_apply(Saml2LoginConfigurer::default(), saml2_login);

        self
    }

    pub fn saml2_logout<F>(mut self, saml2_logout: F) -> Self
    where
        F: FnMut(&mut Saml2LogoutConfigurer<Self>),
    {
        self.get_or_apply(Saml2LogoutConfigurer::new(self.get_context()), saml2_logout);

        self
    }

    pub fn saml2_metadata<F>(mut self, saml2_metadata: F) -> Self
    where
        F: FnMut(&mut Saml2MetadataConfigurer<Self>),
    {
        self.get_or_apply(
            Saml2MetadataConfigurer::new(self.get_context()),
            saml2_metadata,
        );

        self
    }

    pub fn oauth2_login<F>(mut self, oauth2_login: F) -> Self
    where
        F: FnMut(&mut OAuth2LoginConfigurer<Self>),
    {
        self.get_or_apply(OAuth2LoginConfigurer::default(), oauth2_login);

        self
    }

    pub fn oidc_logout<F>(mut self, oidc_logout: F) -> Self
    where
        F: FnMut(&mut OidcLogoutConfigurer<Self>),
    {
        self.get_or_apply(OidcLogoutConfigurer::default(), oidc_logout);

        self
    }

    pub fn oauth2_client<F>(mut self, oauth2_client: F) -> Self
    where
        F: FnMut(&mut OAuth2ClientConfigurer<Self>),
    {
        self.get_or_apply(OAuth2ClientConfigurer::default(), oauth2_client);

        self
    }

    pub fn oauth2_resource_server<F>(mut self, oauth2_resource_server: F) -> Self
    where
        F: FnMut(&mut OAuth2ResourceServerConfigurer<Self>),
    {
        self.get_or_apply(
            OAuth2ResourceServerConfigurer::new(self.get_context()),
            oauth2_resource_server,
        );

        self
    }

    pub fn oauth2_authorization_server<F>(mut self, oauth2_authorization_server: F) -> Self
    where
        F: FnMut(&mut OAuth2AuthorizationServerConfigurer<Self>),
    {
        self.get_or_apply(
            OAuth2AuthorizationServerConfigurer::default(),
            oauth2_authorization_server,
        );

        self
    }

    pub fn one_time_token_login<F>(mut self, one_time_token_login: F) -> Self
    where
        F: FnMut(&mut OneTimeTokenLoginConfigurer<Self>),
    {
        self.get_or_apply(
            OneTimeTokenLoginConfigurer::new(self.get_context()),
            one_time_token_login,
        );

        self
    }

    pub fn redirect_to_https<F>(mut self, redirect_to_https: F) -> Self
    where
        F: FnMut(&mut HttpsRedirectConfigurer<Self>),
    {
        self.get_or_apply(HttpsRedirectConfigurer::default(), redirect_to_https);

        self
    }

    pub fn http_basic<F>(mut self, http_basic: F) -> Self
    where
        F: FnMut(&mut HttpBasicConfigurer<Self>),
    {
        self.get_or_apply(HttpBasicConfigurer::default(), http_basic);

        self
    }

    pub fn password_management<F>(mut self, password_management: F) -> Self
    where
        F: FnMut(&mut PasswordManagementConfigurer<Self>),
    {
        self.get_or_apply(PasswordManagementConfigurer::default(), password_management);

        self
    }

    pub fn authentication_manager(
        mut self,
        authentication_manager: Arc<dyn AuthenticationManager>,
    ) -> Self {
        self.authentication_manager = Some(authentication_manager);
        self
    }

    pub fn security_matchers<F>(mut self, mut security_matchers: F) -> Self
    where
        F: FnMut(&mut RequestMatcherConfigurer),
    {
        security_matchers(&mut self.request_matcher_configurer);
        let security_matcher = self.request_matcher_configurer.or_request_matcher();
        self.security_matcher(security_matcher)
    }

    pub fn web_authn<F>(mut self, web_authn: F) -> Self
    where
        F: FnMut(&mut WebAuthnConfigurer<Self>),
    {
        self.get_or_apply(WebAuthnConfigurer::default(), web_authn);

        self
    }

    pub fn security_matcher(mut self, request_matcher: Arc<dyn RequestMatcher>) -> Self {
        self.request_matcher = request_matcher;
        self
    }

    pub fn security_matcher_with_strs(mut self, patterns: &[&str]) -> Self {
        if let Some(builder) = self.get_shared_object::<Builder>() {
            let matchers = patterns
                .into_iter()
                .map(|s| builder.matcher(None, s))
                .map(|v| Arc::new(v) as Arc<dyn RequestMatcher>)
                .collect::<Vec<_>>();

            self.request_matcher = Arc::new(OrRequestMatcher::new(matchers));
        }

        self
    }

    pub fn add_filter_at<F, F1>(&mut self, filter: F)
    where
        F: HttpFilter,
        F1: HttpFilter,
    {
        self.add_filter_at_offset::<F, F1>(filter, 0);
    }

    pub fn get_authentication_registry(&mut self) -> Option<&mut AuthenticationManagerBuilder> {
        self.base.get_configurer::<AuthenticationManagerBuilder>()
    }
}

// =========================================================================
// Trait implementations
// =========================================================================
impl SecurityBuilder<DefaultSecurityFilterChain> for HttpSecurity {
    fn build(&mut self) -> DefaultSecurityFilterChain {
        assert!(
            !self
                .base
                .base_security_builder
                .building
                .swap(true, Ordering::SeqCst),
            "This object has already been built"
        );

        self.base.build_state = BuildState::INITIALIZING;
        self.before_init();
        self.base.init();
        self.base.build_state = BuildState::CONFIGURING;
        self.before_configure();
        self.base.configure();
        self.base.build_state = BuildState::BUILDING;
        let result = self.perform_build();
        self.base.build_state = BuildState::BUILT;

        result
    }
}

impl BaseConfiguredSecurityBuilderExt<DefaultSecurityFilterChain, Self> for HttpSecurity {
    fn before_init(&mut self) {}

    fn before_configure(&mut self) {
        let manager = match self.authentication_manager.as_ref() {
            Some(manager) => Some(manager.clone()),
            None => self.get_authentication_registry().map(|s| s.build()),
        };

        self.set_shared_object(manager);
    }

    fn perform_build(&mut self) -> DefaultSecurityFilterChain {
        self.filters.sort_by_key(|c| c.order);
        let sorted_filters = self
            .filters
            .iter()
            .map(|c| c.filter.clone())
            .collect::<Vec<_>>();

        DefaultSecurityFilterChain::new(self.request_matcher.clone(), sorted_filters)
    }
}

impl HttpSecurityBuilder<Self> for HttpSecurity {
    fn add_filter<F: HttpFilter>(&mut self, filter: F) {
        let name = std::any::type_name::<F>();
        let order = self.filter_orders.get_order_by_name(name).unwrap_or_else(|| {
            panic!("Filter {name} has no registered order. Use add_filter_before or add_filter_after.")
        });
        self.filters
            .push(OrderedFilter::new(Arc::new(filter), order));
    }

    fn get_configurer<C>(&mut self) -> Option<&mut C>
    where
        C: SecurityConfigurer<DefaultSecurityFilterChain, Self>,
        C: 'static,
    {
        self.base.get_configurer()
    }

    fn remove_configurer<C>(&mut self) -> Option<C>
    where
        C: SecurityConfigurer<DefaultSecurityFilterChain, Self>,
        C: AnyClone + 'static,
    {
        self.base.remove_configurer()
    }

    fn authentication_provider<T>(&mut self, authentication_provider: T)
    where
        T: AuthenticationProvider + 'static,
    {
        self.get_authentication_registry()
            .map(|r| r.authentication_provider(Arc::new(authentication_provider)));
    }

    fn user_details_service<T>(&mut self, user_details_service: T)
    where
        T: UserDetailsService + 'static,
    {
        self.get_authentication_registry()
            .map(|r| r.user_details_service(Arc::new(user_details_service)));
    }

    fn set_shared_object<C>(&mut self, object: C)
    where
        C: AnyClone,
    {
        self.base.set_shared_object(object);
    }

    fn get_shared_object<T>(&self) -> Option<&T>
    where
        T: AnyClone,
    {
        self.base.get_shared_object()
    }

    fn get_mut_shared_object<T>(&mut self) -> Option<&mut T>
    where
        T: AnyClone,
    {
        self.base.get_mut_shared_object()
    }

    fn add_filter_after<F, F1>(&mut self, f: F)
    where
        F: HttpFilter,
        F1: HttpFilter,
    {
        self.add_filter_at_offset::<F, F1>(f, 1);
    }

    fn add_filter_before<F, F1>(&mut self, f: F)
    where
        F: HttpFilter,
        F1: HttpFilter,
    {
        self.add_filter_at_offset::<F, F1>(f, -1);
    }
}

impl Clone for HttpSecurity {
    fn clone(&self) -> Self {
        Self {
            request_matcher_configurer: self.request_matcher_configurer.clone(),
            filters: self.filters.clone(),
            request_matcher: self.request_matcher.clone(),
            filter_orders: self.filter_orders.clone(),
            authentication_manager: self.authentication_manager.clone(),

            base: self.base.clone(),
        }
    }
}

impl Deref for HttpSecurity {
    type Target = BaseConfiguredSecurityBuilder<DefaultSecurityFilterChain, Self>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for HttpSecurity {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[derive(Clone)]
pub struct RequestMatcherConfigurer {
    base: BaseRequestMatcherRegistry<Self>,
}

impl RequestMatcherConfigurer {
    pub fn new() -> Self {
        Self {
            base: Default::default(),
        }
    }

    pub fn set_matchers(&mut self, matchers: Vec<Arc<dyn RequestMatcher>>) {
        self.base._req_matchers = matchers;
    }

    fn or_request_matcher(&self) -> Arc<dyn RequestMatcher> {
        Arc::new(OrRequestMatcher::new(self.base._req_matchers.clone()))
    }
}

impl Deref for RequestMatcherConfigurer {
    type Target = BaseRequestMatcherRegistry<Self>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for RequestMatcherConfigurer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[derive(Clone)]
pub struct OrderedFilter {
    filter: Arc<dyn HttpFilter>,
    order: i32,
}
impl OrderedFilter {
    fn new(filter: Arc<dyn HttpFilter>, order: i32) -> Self {
        Self { filter, order }
    }
}
impl Ordered for OrderedFilter {
    fn order(&self) -> i32 {
        self.order
    }
}
#[async_trait]
impl HttpFilter for OrderedFilter {
    async fn do_filter(
        &self,
        r: &mut dyn HttpRequest,
        w: &mut dyn HttpResponse,
        c: &dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        self.filter.do_filter(r, w, c).await
    }
}
impl Named for OrderedFilter {
    fn name(&self) -> &str {
        "OrderedFilter"
    }
}

impl Display for OrderedFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}(filter={}, order={})",
            self.name(),
            self.filter.name(),
            self.order
        )
    }
}
