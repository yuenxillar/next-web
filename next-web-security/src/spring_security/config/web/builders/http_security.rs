use std::{
    any::{type_name, TypeId},
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, Mutex},
};

use next_web_core::{
    anys::any_map::AnyMap,
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
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        web::{
            builders::filter_order_registration::FilterOrderRegistration,
            configurers::{
                authorize_http_requests_configurer::{
                    AuthorizationManagerRequestMatcherRegistry, AuthorizeHttpRequestsConfigurer,
                },
                base_authentication_filter_configurer::AuthenticationFilterConfigurer,
                form_login_configurer::FormLoginConfigurer,
                logout_configurer::LogoutConfigurer,
                AnonymousConfigurer, CorsConfigurer, CsrfConfigurer,
                ErrorHandlingConfigurer, HeadersConfigurer, HttpBasicConfigurer,
                HttpsRedirectConfigurer, JeeConfigurer,
                OAuth2AuthorizationServerConfigurer, OAuth2ClientConfigurer,
                OAuth2LoginConfigurer, OAuth2ResourceServerConfigurer,
                OneTimeTokenLoginConfigurer, PasswordManagementConfigurer,
                PortMapperConfigurer, RememberMeConfigurer, RequestCacheConfigurer,
                Saml2LoginConfigurer, Saml2LogoutConfigurer,
                Saml2MetadataConfigurer, SessionManagementConfigurer, X509Configurer,
            },
            http_security_builder::HttpSecurityBuilder,
        },
    },
    core::userdetails::user_details_service::UserDetailsService,
    web::{
        access::intercept::AuthorizationFilter,
        authentication::{
            ui::default_login_page_generating_filter::DefaultLoginPageGeneratingFilter,
            username_password_authentication_filter::UsernamePasswordAuthenticationFilter,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
        util::matcher::{AnyRequestMatcher, OrRequestMatcher, RequestMatcher},
    },
};

// =========================================================================
// Lifecycle dispatch macro — generates `call_init` / `call_configure`.
// Add new configurer types to the invocations inside the impl block.
// =========================================================================

macro_rules! lifecycle_dispatch {
    ($fn:ident, $method:ident, [$($ty:ty),+ $(,)?]) => {
        fn $fn(&mut self, tid: std::any::TypeId) {
            $(
                if tid == std::any::TypeId::of::<$ty>() {
                    if let Some(mut c) = self.lookup_configurer::<$ty>() {
                        c.$method(self);
                        self.configurers.lock().unwrap().insert(tid, Box::new(c));
                    }
                    return;
                }
            )+
        }
    };
}

// =========================================================================
// HttpSecurity — mirrors Java's HttpSecurity which extends
// AbstractConfiguredSecurityBuilder. Configurers are stored in a shared
// type-erased map (the equivalent of Java's LinkedHashMap<Class<?>, List>).
// No individual Option<Configurer> fields exist in this struct.
// =========================================================================

pub struct HttpSecurity {
    // --- Configurer storage (equivalent to AbstractConfiguredSecurityBuilder.configurers) ---
    // Wrapped in Arc<Mutex<>> so that Clone shares the same map — configurers
    // accumulated during the builder chain are visible to all clones.
    configurers: Arc<Mutex<HashMap<TypeId, Box<dyn AnyClone>>>>,

    // --- Fields from HttpSecurity.java (lines 151–159) ---
    request_matcher_configurer: RequestMatcherConfigurer,
    filters: Vec<OrderedFilter>,
    request_matcher: Arc<dyn RequestMatcher>,
    filter_orders: FilterOrderRegistration,
    authentication_manager: Option<Arc<dyn AuthenticationManager>>,

    // --- Additional state needed in Rust ---
    application_context: ApplicationContext,
    authentication_builder: AuthenticationManagerBuilder,
    default_login_page_generating_filter: Option<DefaultLoginPageGeneratingFilter>,

    /// AuthorizeHttpRequestsConfigurer kept as a field because its
    /// AuthorizationManager is built from the registry at build time
    /// (not inside the configurer's configure() lifecycle).
    authorize_http_requests_configurer: Option<AuthorizeHttpRequestsConfigurer<HttpSecurity>>,

    /// Shared objects (equivalent to AbstractConfiguredSecurityBuilder.sharedObjects).
    shared_objects: Arc<Mutex<HashMap<String, Box<dyn AnyClone>>>>,
}

impl HttpSecurity {
    pub fn new(
        authentication_builder: AuthenticationManagerBuilder,
        shared_objects: AnyMap,
    ) -> Self {
        let mut http = Self {
            configurers: Arc::new(Mutex::new(HashMap::new())),
            filter_orders: Default::default(),
            request_matcher: Arc::new(AnyRequestMatcher),
            filters: Vec::new(),
            request_matcher_configurer: RequestMatcherConfigurer::new(),
            authentication_manager: None,
            application_context: ApplicationContext::default(),
            authentication_builder,
            default_login_page_generating_filter: Some(DefaultLoginPageGeneratingFilter::new(
                None,
            )),
            authorize_http_requests_configurer: None,
            shared_objects: Arc::new(Mutex::new(HashMap::new())),
        };

        shared_objects.for_each(|name, object| {
            http.set_shared_object(name.to_string(), object.to_owned());
        });

        http
    }

    fn get_context(&self) -> &ApplicationContext {
        &self.application_context
    }

    // ------------------------------------------------------------------
    // getOrApply — the central configurer management pattern from Java.
    // Every configurer DSL method delegates to this single function.
    // ------------------------------------------------------------------

    /// Equivalent to Java's `getOrApply(C configurer)`.
    /// If a configurer of type `C` is already registered, clone and return it.
    /// Otherwise, register the given configurer and return it.
    fn get_or_apply<C>(&self, configurer: C) -> C
    where
        C: AnyClone + Clone + 'static,
    {
        let mut map = self.configurers.lock().expect("configurer lock");
        let key = TypeId::of::<C>();
        if let Some(existing) = map.get(&key) {
            clone_from_map::<C>(existing)
        } else {
            map.insert(key, Box::new(configurer.clone()));
            configurer
        }
    }

    // ------------------------------------------------------------------
    // Configurer access (reads from the shared map).
    // ------------------------------------------------------------------

    /// Look up a configurer by type from the shared map.
    fn lookup_configurer<C: AnyClone + Clone + 'static>(&self) -> Option<C> {
        let map = self.configurers.lock().expect("configurer lock");
        map.get(&TypeId::of::<C>())
            .map(|boxed| clone_from_map::<C>(boxed))
    }

    /// Remove a configurer by type from the shared map.
    fn remove_from_map<C: 'static>(&self) {
        let mut map = self.configurers.lock().expect("configurer lock");
        map.remove(&TypeId::of::<C>());
    }

    // ------------------------------------------------------------------
    // Build lifecycle — mirrors AbstractConfiguredSecurityBuilder.doBuild()
    // ------------------------------------------------------------------

    fn do_build(&mut self) -> DefaultSecurityFilterChain {
        self.before_configure();
        self.init_configurers();
        self.configure_configurers();
        self.sync_framework_filters();
        self.filters.sort_by_key(Ordered::order);
        let filters = self
            .filters
            .iter()
            .map(|o| o.filter.clone())
            .collect::<Vec<_>>();
        DefaultSecurityFilterChain::new(self.request_matcher.clone(), filters)
    }

    fn before_configure(&mut self) {
        let manager = match &self.authentication_manager {
            Some(m) => m.clone(),
            None => {
                let m = self.authentication_builder.build();
                self.authentication_manager = Some(m.clone());
                m
            }
        };
        self.set_shared_object("authentication_manager", manager);
    }

    /// Iterate the configurer map and call init() on every known configurer.
    /// Mirrors Java's `init()` loop in AbstractConfiguredSecurityBuilder.
    fn init_configurers(&mut self) {
        let type_ids: Vec<TypeId> = {
            self.configurers
                .lock()
                .expect("lock")
                .keys()
                .copied()
                .collect()
        };
        for tid in type_ids {
            self.call_init(tid);
        }
    }

    /// Iterate the configurer map and call configure() on every known configurer.
    fn configure_configurers(&mut self) {
        let type_ids: Vec<TypeId> = {
            self.configurers
                .lock()
                .expect("lock")
                .keys()
                .copied()
                .collect()
        };
        for tid in type_ids {
            self.call_configure(tid);
        }
    }

    lifecycle_dispatch!(call_init, init, [
        CsrfConfigurer<Self>,
        FormLoginConfigurer<Self>,
        LogoutConfigurer<Self>,
        AnonymousConfigurer<Self>,
        HttpBasicConfigurer<Self>,
        RememberMeConfigurer<Self>,
        PortMapperConfigurer<Self>,
        RequestCacheConfigurer<Self>,
        HttpsRedirectConfigurer<Self>,
    ]);

    lifecycle_dispatch!(call_configure, configure, [
        CsrfConfigurer<Self>,
        FormLoginConfigurer<Self>,
        LogoutConfigurer<Self>,
        AnonymousConfigurer<Self>,
        HttpBasicConfigurer<Self>,
        RememberMeConfigurer<Self>,
        RequestCacheConfigurer<Self>,
        HeadersConfigurer<Self>,
        CorsConfigurer<Self>,
        PortMapperConfigurer<Self>,
        X509Configurer<Self>,
        OAuth2LoginConfigurer<Self>,
        OAuth2ClientConfigurer<Self>,
        OAuth2ResourceServerConfigurer<Self>,
        OAuth2AuthorizationServerConfigurer<Self>,
        OneTimeTokenLoginConfigurer<Self>,
        PasswordManagementConfigurer<Self>,
        ErrorHandlingConfigurer<Self>,
        HttpsRedirectConfigurer<Self>,
        JeeConfigurer<Self>,
        Saml2LoginConfigurer<Self>,
        Saml2LogoutConfigurer<Self>,
        Saml2MetadataConfigurer<Self>,
    ]);

    // ------------------------------------------------------------------
    // Framework filter synchronization
    // ------------------------------------------------------------------

    fn sync_framework_filters(&mut self) {
        // DefaultLoginPageGeneratingFilter — not managed by any configurer
        if let Some(lp) = &self.default_login_page_generating_filter {
            if lp.is_enabled() {
                self.add_filter_internal(lp.clone(), None);
            }
        }

        // AuthorizeHttpRequestsConfigurer → AuthorizationFilter
        if let Some(c) = &self.authorize_http_requests_configurer {
            let mgr = c.get_registry().create_authorization_manager();
            self.add_filter_internal(AuthorizationFilter::new(mgr), None);
        }
    }

    // ------------------------------------------------------------------
    // Filter helpers
    // ------------------------------------------------------------------

    fn add_filter_at_offset<F, F1>(&mut self, filter: F, offset: i32)
    where
        F: HttpFilter,
        F1: HttpFilter,
    {
        let name = std::any::type_name::<F1>();
        let base = self
            .filter_orders
            .get_order_by_name(name)
            .unwrap_or_else(|| panic!("Filter {} has no registered order", name));
        let order = base + offset;
        self.filter_orders.put::<F>(order);
        self.filters
            .push(OrderedFilter::new(Arc::new(filter), order));
    }

    fn add_filter_internal<F: HttpFilter + 'static>(&mut self, filter: F, order: Option<i32>) {
        let f = Arc::new(filter);
        let o = order.unwrap_or_else(|| {
            self.filter_orders
                .get_order_by_name(std::any::type_name::<F>())
                .unwrap_or_else(|| {
                    self.filters.iter().map(Ordered::order).max().unwrap_or(0) + 100
                })
        });
        self.filters.push(OrderedFilter::new(f, o));
    }
}

// =========================================================================
// Configurer DSL methods — each delegates to get_or_apply.
// No individual Option<> fields; no per-type storage logic.
// =========================================================================

impl HttpSecurity {
    pub fn csrf<F>(mut self, f: F) -> Self
    where F: FnOnce(CsrfConfigurer<Self>),
    {
        let c = self.get_or_apply(CsrfConfigurer::new(self.get_context()));
        f(c.clone());
        self
    }

    pub fn session_management<F>(self, f: F) -> Self
    where F: FnOnce(SessionManagementConfigurer<Self>),
    {
        f(SessionManagementConfigurer::default());
        self
    }

    pub fn authorize_http_requests<F>(mut self, f: F) -> Self
    where F: FnOnce(AuthorizationManagerRequestMatcherRegistry),
    {
        let c = AuthorizeHttpRequestsConfigurer::new(self.get_context());
        f(c.get_registry());
        self.authorize_http_requests_configurer = Some(c);
        self
    }

    pub fn form_login<F>(mut self, f: F) -> Self
    where F: FnOnce(FormLoginConfigurer<Self>),
    {
        let mut c = self.get_or_apply(FormLoginConfigurer::default());
        f(c.clone());
        c.init_default_login_filter(&mut self);
        self
    }

    pub fn logout<F>(mut self, f: F) -> Self
    where F: FnOnce(LogoutConfigurer<Self>),
    {
        let c = self.get_or_apply(LogoutConfigurer::default());
        f(c.clone());
        self
    }

    pub fn error_handling<F>(mut self, f: F) -> Self
    where F: FnOnce(ErrorHandlingConfigurer<Self>),
    { let c = self.get_or_apply(ErrorHandlingConfigurer::default()); f(c.clone()); self }

    pub fn request_cache<F>(mut self, f: F) -> Self
    where F: FnOnce(RequestCacheConfigurer<Self>),
    { let c = self.get_or_apply(RequestCacheConfigurer::default()); f(c.clone()); self }

    pub fn anonymous<F>(mut self, f: F) -> Self
    where F: FnOnce(AnonymousConfigurer<Self>),
    { let c = self.get_or_apply(AnonymousConfigurer::default()); f(c.clone()); self }

    pub fn http_basic<F>(mut self, f: F) -> Self
    where F: FnOnce(HttpBasicConfigurer<Self>),
    { let c = self.get_or_apply(HttpBasicConfigurer::default()); f(c.clone()); self }

    pub fn remember_me<F>(mut self, f: F) -> Self
    where F: FnOnce(RememberMeConfigurer<Self>),
    { let c = self.get_or_apply(RememberMeConfigurer::default()); f(c.clone()); self }

    pub fn headers<F>(mut self, f: F) -> Self
    where F: FnOnce(HeadersConfigurer<Self>),
    { let c = self.get_or_apply(HeadersConfigurer::default()); f(c.clone()); self }

    pub fn cors<F>(mut self, f: F) -> Self
    where F: FnOnce(CorsConfigurer<Self>),
    { let c = self.get_or_apply(CorsConfigurer::default()); f(c.clone()); self }

    pub fn port_mapper<F>(mut self, f: F) -> Self
    where F: FnOnce(PortMapperConfigurer<Self>),
    { let c = self.get_or_apply(PortMapperConfigurer::default()); f(c.clone()); self }

    pub fn x509<F>(mut self, f: F) -> Self
    where F: FnOnce(X509Configurer<Self>),
    { let c = self.get_or_apply(X509Configurer::default()); f(c.clone()); self }

    pub fn oauth2_login<F>(mut self, f: F) -> Self
    where F: FnOnce(OAuth2LoginConfigurer<Self>),
    { let c = self.get_or_apply(OAuth2LoginConfigurer::default()); f(c.clone()); self }

    pub fn oauth2_client<F>(mut self, f: F) -> Self
    where F: FnOnce(OAuth2ClientConfigurer<Self>),
    { let c = self.get_or_apply(OAuth2ClientConfigurer::default()); f(c.clone()); self }

    pub fn oauth2_resource_server<F>(mut self, f: F) -> Self
    where F: FnOnce(OAuth2ResourceServerConfigurer<Self>),
    { let c = self.get_or_apply(OAuth2ResourceServerConfigurer::default()); f(c.clone()); self }

    pub fn oauth2_authorization_server<F>(mut self, f: F) -> Self
    where F: FnOnce(OAuth2AuthorizationServerConfigurer<Self>),
    { let c = self.get_or_apply(OAuth2AuthorizationServerConfigurer::default()); f(c.clone()); self }

    pub fn one_time_token_login<F>(mut self, f: F) -> Self
    where F: FnOnce(OneTimeTokenLoginConfigurer<Self>),
    { let c = self.get_or_apply(OneTimeTokenLoginConfigurer::default()); f(c.clone()); self }

    pub fn password_management<F>(mut self, f: F) -> Self
    where F: FnOnce(PasswordManagementConfigurer<Self>),
    { let c = self.get_or_apply(PasswordManagementConfigurer::default()); f(c.clone()); self }

    pub fn redirect_to_https<F>(mut self, f: F) -> Self
    where F: FnOnce(HttpsRedirectConfigurer<Self>),
    { let c = self.get_or_apply(HttpsRedirectConfigurer::default()); f(c.clone()); self }

    pub fn jee<F>(mut self, f: F) -> Self
    where F: FnOnce(JeeConfigurer<Self>),
    { let c = self.get_or_apply(JeeConfigurer::default()); f(c.clone()); self }

    pub fn saml2_login<F>(mut self, f: F) -> Self
    where F: FnOnce(Saml2LoginConfigurer<Self>),
    { let c = self.get_or_apply(Saml2LoginConfigurer::default()); f(c.clone()); self }

    pub fn saml2_logout<F>(mut self, f: F) -> Self
    where F: FnOnce(Saml2LogoutConfigurer<Self>),
    { let c = self.get_or_apply(Saml2LogoutConfigurer::default()); f(c.clone()); self }

    pub fn saml2_metadata<F>(mut self, f: F) -> Self
    where F: FnOnce(Saml2MetadataConfigurer<Self>),
    { let c = self.get_or_apply(Saml2MetadataConfigurer::default()); f(c.clone()); self }

    pub fn authentication_manager(mut self, m: Arc<dyn AuthenticationManager>) -> Self {
        self.authentication_manager = Some(m);
        self
    }

    pub fn security_matcher(mut self, m: Arc<dyn RequestMatcher>) -> Self {
        self.request_matcher = m;
        self
    }

    pub fn security_matchers<F>(mut self, f: F) -> Self
    where F: FnOnce(&mut RequestMatcherConfigurer),
    {
        f(&mut self.request_matcher_configurer);
        if let Some(m) = self.request_matcher_configurer.build_matcher() {
            self.request_matcher = m;
        }
        self
    }

    pub fn add_filter_at<F, F1>(&mut self, filter: F)
    where F: HttpFilter, F1: HttpFilter,
    {
        self.add_filter_at_offset::<F, F1>(filter, 0);
    }

    pub fn oidc_logout<F>(mut self, _f: F) -> Self {
        todo!("oidc_logout")
    }
}

// =========================================================================
// Trait implementations
// =========================================================================

impl SecurityBuilder<DefaultSecurityFilterChain> for HttpSecurity {
    fn build(&self) -> DefaultSecurityFilterChain {
        let mut http = self.clone(); // shallow — shares configurer map via Arc
        if http.authentication_manager.is_none() {
            http.authentication_manager = Some(http.authentication_builder.build());
        }
        http.do_build()
    }
}

impl SecurityBuilder<Self> for HttpSecurity {
    fn build(&self) -> Self { self.clone() }
}

/// Marker impl — satisfies the `SecurityBuilder<FormLoginConfigurer<H>>` bound
/// required by FormLoginConfigurer's SecurityConfigurer, enabling it to live in
/// the type-erased configurer map. Never actually called.
impl SecurityBuilder<FormLoginConfigurer<HttpSecurity>> for HttpSecurity {
    fn build(&self) -> FormLoginConfigurer<HttpSecurity> {
        unreachable!("SecurityBuilder<FormLoginConfigurer> is a trait-bound shim only")
    }
}

impl AuthenticationFilterConfigurer<Self> for HttpSecurity {
    fn login_processing_url(&mut self, _url: &str) {}
    fn login_page(&mut self, _page: &str) {}
}

impl HttpSecurityBuilder<Self> for HttpSecurity {
    fn add_filter<F: HttpFilter>(&mut self, filter: F) {
        let n = std::any::type_name::<F>();
        let o = self.filter_orders.get_order_by_name(n).unwrap_or_else(|| {
            panic!("Filter {n} has no registered order. Use add_filter_before or add_filter_after.")
        });
        self.filters.push(OrderedFilter::new(Arc::new(filter), o));
    }

    fn get_configurer<T>(&self) -> Option<T>
    where T: SecurityConfigurer<DefaultSecurityFilterChain, Self>,
    {
        // Bridge to the map for types that csrf_configurer.rs queries.
        // (These are always Clone+'static at the concrete call site.)
        let tn = std::any::type_name::<T>();
        if tn == std::any::type_name::<LogoutConfigurer<Self>>() {
            self.lookup_configurer::<LogoutConfigurer<Self>>()
                .map(|c| clone_as::<LogoutConfigurer<Self>, T>(&c))
        } else if tn == std::any::type_name::<ErrorHandlingConfigurer<Self>>() {
            self.lookup_configurer::<ErrorHandlingConfigurer<Self>>()
                .map(|c| clone_as::<ErrorHandlingConfigurer<Self>, T>(&c))
        } else {
            None
        }
    }

    fn remove_configurer<T>(&mut self)
    where T: SecurityConfigurer<DefaultSecurityFilterChain, Self>,
    {
        let tn = std::any::type_name::<T>();
        if tn == std::any::type_name::<CsrfConfigurer<Self>>() {
            self.remove_from_map::<CsrfConfigurer<Self>>();
        } else if tn == std::any::type_name::<LogoutConfigurer<Self>>() {
            self.remove_from_map::<LogoutConfigurer<Self>>();
        }
    }

    fn authentication_provider<T>(&mut self, p: T) -> Self
    where T: AuthenticationProvider + 'static,
    {
        self.authentication_builder.authentication_provider(Arc::new(p));
        self.clone()
    }

    fn user_details_service<T>(&mut self, u: T) -> Self
    where T: UserDetailsService + 'static,
    {
        self.authentication_builder.user_details_service(Arc::new(u));
        self.clone()
    }

    fn get_shared_object<T>(&self) -> Option<&T> {
        if type_name::<T>() == type_name::<DefaultLoginPageGeneratingFilter>() {
            self.default_login_page_generating_filter
                .as_ref()
                .map(|f| unsafe { &*(f as *const _ as *const T) })
        } else {
            None
        }
    }

    fn get_mut_shared_object<T>(&mut self) -> Option<&mut T> {
        if type_name::<T>() == type_name::<DefaultLoginPageGeneratingFilter>() {
            self.default_login_page_generating_filter
                .as_mut()
                .map(|f| unsafe { &mut *(f as *mut _ as *mut T) })
        } else {
            None
        }
    }

    fn add_filter_after<F, F1>(&mut self, f: F)
    where F: HttpFilter, F1: HttpFilter,
    { self.add_filter_at_offset::<F, F1>(f, 1); }

    fn add_filter_before<F, F1>(&mut self, f: F)
    where F: HttpFilter, F1: HttpFilter,
    { self.add_filter_at_offset::<F, F1>(f, -1); }

    fn set_shared_object<N, C>(&self, n: N, o: C)
    where N: Into<Cow<'static, str>>, C: AnyClone,
    {
        self.shared_objects
            .lock()
            .expect("lock")
            .insert(n.into().into_owned(), Box::new(o));
    }

    fn authentication_manager(&self) -> Option<Arc<dyn AuthenticationManager>> {
        self.authentication_manager.clone()
    }
}

impl Clone for HttpSecurity {
    fn clone(&self) -> Self {
        Self {
            configurers: Arc::clone(&self.configurers), // shared!
            shared_objects: Arc::clone(&self.shared_objects), // shared!
            request_matcher_configurer: self.request_matcher_configurer.clone(),
            filters: self.filters.clone(),
            request_matcher: self.request_matcher.clone(),
            filter_orders: self.filter_orders.clone(),
            authentication_manager: self.authentication_manager.clone(),
            application_context: self.application_context.clone(),
            authentication_builder: self.authentication_builder.clone(),
            default_login_page_generating_filter: self.default_login_page_generating_filter.clone(),
            authorize_http_requests_configurer: self.authorize_http_requests_configurer.clone(),
        }
    }
}

impl Default for HttpSecurity {
    fn default() -> Self {
        Self::new(AuthenticationManagerBuilder::new(), AnyMap::new())
    }
}

// =========================================================================
// Supporting types
// =========================================================================

#[derive(Clone)]
pub struct RequestMatcherConfigurer {
    matchers: Vec<Arc<dyn RequestMatcher>>,
}

impl RequestMatcherConfigurer {
    pub fn new() -> Self { Self { matchers: Vec::new() } }
    pub fn request_matchers(&mut self, m: Vec<Arc<dyn RequestMatcher>>) -> &mut Self {
        self.matchers.extend(m);
        self
    }
    pub fn build_matcher(&self) -> Option<Arc<dyn RequestMatcher>> {
        match self.matchers.len() {
            0 => None,
            1 => Some(self.matchers[0].clone()),
            _ => Some(Arc::new(OrRequestMatcher::new(self.matchers.clone()))),
        }
    }
}

#[derive(Clone)]
pub struct OrderedFilter {
    filter: Arc<dyn HttpFilter>,
    order: i32,
}
impl OrderedFilter {
    fn new(filter: Arc<dyn HttpFilter>, order: i32) -> Self { Self { filter, order } }
}
impl Ordered for OrderedFilter {
    fn order(&self) -> i32 { self.order }
}
#[async_trait]
impl HttpFilter for OrderedFilter {
    async fn do_filter(&self, r: &mut dyn HttpRequest, w: &mut dyn HttpResponse, c: &dyn HttpFilterChain) -> Result<(), BoxError> {
        self.filter.do_filter(r, w, c).await
    }
}
impl Named for OrderedFilter {
    fn name(&self) -> &str { "OrderedFilter" }
}

/// Clone a configurer out of the type-erased map.
/// Extracts &C from the dyn reference (via raw pointer cast) then clones.
/// Safe because TypeId already verified the stored type matches C.
fn clone_from_map<C: Clone + 'static>(entry: &Box<dyn AnyClone>) -> C {
    // entry: &Box<dyn AnyClone>
    // **entry: dyn AnyClone   (the trait object)
    // &**entry: &dyn AnyClone
    // The data pointer inside the fat reference points to a C value.
    let dyn_ref: &dyn AnyClone = entry.as_ref();
    let ptr: *const C = (dyn_ref as *const dyn AnyClone) as *const C;
    unsafe { (*ptr).clone() }
}

fn clone_as<S: Clone, T>(s: &S) -> T {
    let b = Box::new(s.clone());
    unsafe { *Box::from_raw(Box::into_raw(b) as *mut T) }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use crate::{
        config::security_builder::SecurityBuilder,
        web::{default_security_filter_chain::DefaultSecurityFilterChain, security_filter_chain::SecurityFilterChain},
    };
    use super::HttpSecurity;

    #[test]
    fn default_http_security_builds_an_empty_chain() {
        let chain: DefaultSecurityFilterChain = HttpSecurity::default().build();
        assert_eq!(chain.get_filters().len(), 0);
    }

    #[test]
    fn authorize_http_requests_adds_an_authorization_filter() {
        let chain: DefaultSecurityFilterChain = HttpSecurity::default()
            .authorize_http_requests(|mut r| { r.any_request().authenticated(); })
            .build();
        assert_eq!(chain.get_filters().len(), 1);
    }

    #[test]
    fn form_login_registers_login_filters() {
        let chain: DefaultSecurityFilterChain = HttpSecurity::default()
            .form_login(|_| {})
            .build();
        assert_eq!(chain.get_filters().len(), 2);
    }
}
