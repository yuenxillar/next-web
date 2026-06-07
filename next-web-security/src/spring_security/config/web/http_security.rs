use std::{
    any::type_name,
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
                CsrfConfigurer, SessionManagementConfigurer,
            },
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::{
        access::intercept::AuthorizationFilter,
        authentication::{
            logout::LogoutFilter,
            ui::default_login_page_generating_filter::DefaultLoginPageGeneratingFilter,
            username_password_authentication_filter::UsernamePasswordAuthenticationFilter,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
        util::matcher::{AnyRequestMatcher, RequestMatcher},
    },
};

pub struct HttpSecurity {
    request_matcher_configurer: RequestMatcherConfigurer,
    filters: Vec<OrderedFilter>,
    request_matcher: Arc<dyn RequestMatcher>,
    filter_orders: FilterOrderRegistration,
    authentication_manager: Option<Arc<dyn AuthenticationManager>>,

    application_context: ApplicationContext,
    authentication_builder: AuthenticationManagerBuilder,
    default_login_page_generating_filter: Option<DefaultLoginPageGeneratingFilter>,
    authorize_http_requests_configurer: Option<AuthorizeHttpRequestsConfigurer<HttpSecurity>>,
    form_login_configurer: Option<FormLoginConfigurer<HttpSecurity>>,
    logout_configurer: Option<LogoutConfigurer<HttpSecurity>>,
    csrf_configurer: Option<CsrfConfigurer<Self>>,
    shared_objects: Arc<Mutex<HashMap<String, Box<dyn AnyClone>>>>,
}

impl HttpSecurity {
    pub fn new(
        authentication_builder: AuthenticationManagerBuilder,
        shared_objects: AnyMap,
    ) -> Self {
        let http_security = Self {
            filter_orders: Default::default(),
            request_matcher: Arc::new(AnyRequestMatcher),
            filters: Vec::new(),
            request_matcher_configurer: RequestMatcherConfigurer::new(),
            authentication_manager: None,
            application_context: ApplicationContext::default(),
            authentication_builder,
            default_login_page_generating_filter: Some(DefaultLoginPageGeneratingFilter::new(None)),
            authorize_http_requests_configurer: None,
            form_login_configurer: None,
            logout_configurer: None,
            csrf_configurer: None,
            shared_objects: Arc::new(Mutex::new(HashMap::new())),
        };

        shared_objects.for_each(|name, object| {
            http_security.set_shared_object(name.to_string(), object.to_owned());
        });

        http_security
    }

    fn get_context(&self) -> &ApplicationContext {
        &self.application_context
    }

    fn sync_framework_filters(&mut self) {
        if let Some(form_login) = &self.form_login_configurer {
            let mut filter = next_web_core::traits::required::Required::<
                UsernamePasswordAuthenticationFilter,
            >::get_object(form_login)
            .clone();
            if let Some(authentication_manager) = &self.authentication_manager {
                next_web_core::traits::required::Required::<crate::web::authentication::base_authentication_processing_filter::AbstractAuthenticationProcessingFilter>::get_mut_object(&mut filter)
                    .set_authentication_manager(authentication_manager.clone());
            }
            self.add_filter_internal(filter, None);
        }

        if let Some(login_page_filter) = &self.default_login_page_generating_filter {
            if login_page_filter.is_enabled() {
                self.add_filter_internal(login_page_filter.clone(), None);
            }
        }

        if let Some(logout_configurer) = &self.logout_configurer {
            // let mut filter = LogoutFilter::default();
            // if let Some(url) = logout_configurer.get_logout_success_url() {
            //     filter.set_logout_success_url(url);
            // }
            // self.add_filter_internal(filter, None);
            unimplemented!("logout_configurer unimplemented!")
        }

        if let Some(configurer) = &self.authorize_http_requests_configurer {
            let manager = configurer.get_registry().create_authorization_manager();
            self.add_filter_internal(AuthorizationFilter::new(manager), None);
        }
    }

    fn add_filter_internal<F: HttpFilter + 'static>(&mut self, filter: F, order: Option<i32>) {
        let filter = Arc::new(filter);
        let computed_order = order.unwrap_or_else(|| {
            self.filter_orders
                .get_order_by_name(std::any::type_name::<F>())
                .unwrap_or_else(|| self.filters.iter().map(Ordered::order).max().unwrap_or(0) + 100)
        });

        self.filters
            .push(OrderedFilter::new(filter, computed_order));
    }

    fn perform_build(&mut self) -> DefaultSecurityFilterChain {
        self.sync_framework_filters();
        self.filters.sort_by_key(Ordered::order);
        let filters = self
            .filters
            .iter()
            .map(|ordered| ordered.filter.clone())
            .collect::<Vec<_>>();
        DefaultSecurityFilterChain::new(self.request_matcher.clone(), filters)
    }
}

impl HttpSecurity {
    pub fn csrf<F>(mut self, csrf_configurer: F) -> Self
    where
        F: FnOnce(CsrfConfigurer<Self>),
    {
        let configurer = self
            .csrf_configurer
            .clone()
            .unwrap_or_else(|| CsrfConfigurer::new(self.get_context()));
        csrf_configurer(configurer.clone());
        self.csrf_configurer = Some(configurer);
        self
    }

    pub fn session_management<F>(mut self, session_management_configurer: F) -> Self
    where
        F: FnOnce(SessionManagementConfigurer<Self>),
    {
        session_management_configurer(SessionManagementConfigurer::default());

        self
    }

    pub fn authorize_http_requests<F>(mut self, authorize_http_requests_configurer: F) -> Self
    where
        F: FnOnce(AuthorizationManagerRequestMatcherRegistry),
    {
        let configurer = self
            .authorize_http_requests_configurer
            .clone()
            .unwrap_or_else(|| AuthorizeHttpRequestsConfigurer::new(self.get_context()));
        authorize_http_requests_configurer(configurer.get_registry());
        self.authorize_http_requests_configurer = Some(configurer);
        self
    }

    pub fn form_login<F>(mut self, form_login: F) -> Self
    where
        F: FnOnce(FormLoginConfigurer<HttpSecurity>),
    {
        let mut configurer = self.form_login_configurer.clone().unwrap_or_default();
        form_login(configurer.clone());
        configurer.init_default_login_filter(&mut self);
        self.form_login_configurer = Some(configurer);
        self
    }

    pub fn logout<F>(mut self, logout: F) -> Self
    where
        F: FnOnce(LogoutConfigurer<HttpSecurity>),
    {
        let configurer = self.logout_configurer.clone().unwrap_or_default();
        logout(configurer.clone());
        self.logout_configurer = Some(configurer);
        self
    }
}

impl SecurityBuilder<DefaultSecurityFilterChain> for HttpSecurity {
    fn build(&self) -> DefaultSecurityFilterChain {
        let mut http = self.clone();
        if http.authentication_manager.is_none() {
            http.authentication_manager = Some(http.authentication_builder.build());
        }
        http.perform_build()
    }
}

impl SecurityBuilder<Self> for HttpSecurity {
    fn build(&self) -> Self {
        self.clone()
    }
}

impl AuthenticationFilterConfigurer<Self> for HttpSecurity {
    fn login_processing_url(&mut self, _login_processing_url: &str) {}

    fn login_page(&mut self, _login_page: &str) {}
}

impl HttpSecurityBuilder<Self> for HttpSecurity {
    fn add_filter<F: HttpFilter>(&mut self, filter: F) {}

    fn get_configurer<T>(&self) -> Option<T>
    where
        T: SecurityConfigurer<DefaultSecurityFilterChain, Self>,
    {
        None
    }

    fn get_shared_object<T>(&self) -> Option<&T> {
        if type_name::<T>() == type_name::<DefaultLoginPageGeneratingFilter>() {
            let filter = self.default_login_page_generating_filter.as_ref()?;
            let ptr = filter as *const DefaultLoginPageGeneratingFilter as *const T;
            return Some(unsafe { &*ptr });
        }

        None
    }

    fn get_mut_shared_object<T>(&mut self) -> Option<&mut T> {
        if type_name::<T>() == type_name::<DefaultLoginPageGeneratingFilter>() {
            let filter = self.default_login_page_generating_filter.as_mut()?;
            let ptr = filter as *mut DefaultLoginPageGeneratingFilter as *mut T;
            return Some(unsafe { &mut *ptr });
        }

        None
    }

    fn add_filter_after<F, F1>(mut self, filter: F, _after_filter: F1) -> Self
    where
        F: HttpFilter,
        F1: HttpFilter,
    {
        let _ = filter;
        self
    }

    fn add_filter_before<F, F1>(mut self, filter: F, _before_filter: F1) -> Self
    where
        F: HttpFilter,
        F1: HttpFilter,
    {
        let _ = filter;
        self
    }

    fn set_shared_object<N, C>(&self, name: N, object: C)
    where
        N: Into<Cow<'static, str>>,
        C: AnyClone,
    {
        self.shared_objects
            .lock()
            .expect("shared object lock poisoned")
            .insert(name.into().into_owned(), Box::new(object));
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
            application_context: self.application_context.clone(),
            authentication_builder: self.authentication_builder.clone(),
            default_login_page_generating_filter: self.default_login_page_generating_filter.clone(),
            authorize_http_requests_configurer: self.authorize_http_requests_configurer.clone(),
            form_login_configurer: self.form_login_configurer.clone(),
            logout_configurer: self.logout_configurer.clone(),
            csrf_configurer: self.csrf_configurer.clone(),
            shared_objects: self.shared_objects.clone(),
        }
    }
}

impl Default for HttpSecurity {
    fn default() -> Self {
        Self::new(AuthenticationManagerBuilder::new(), AnyMap::new())
    }
}

#[derive(Clone)]
pub struct RequestMatcherConfigurer {
    matchers: Vec<Arc<dyn RequestMatcher>>,
}

impl RequestMatcherConfigurer {
    pub fn new() -> Self {
        Self {
            matchers: Vec::new(),
        }
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
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        todo!()
    }
}

impl Named for OrderedFilter {
    fn name(&self) -> &str {
        "OrderedFilter"
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::security_builder::SecurityBuilder,
        web::{
            default_security_filter_chain::DefaultSecurityFilterChain,
            security_filter_chain::SecurityFilterChain,
        },
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
            .authorize_http_requests(|mut registry| {
                registry.any_request().authenticated();
            })
            .build();

        assert_eq!(chain.get_filters().len(), 1);
    }

    #[test]
    fn form_login_registers_login_filters() {
        let chain: DefaultSecurityFilterChain = HttpSecurity::default().form_login(|_| {}).build();
        assert_eq!(chain.get_filters().len(), 2);
    }
}
