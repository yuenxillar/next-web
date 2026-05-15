use std::{
    any::Any,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use next_web_core::{traits::required::Required, util::http_method::HttpMethod, ApplicationContext};

use crate::access::hierarchicalroles::role_hierarchy::RoleHierarchy;
use crate::authorization::authorization_decision::AuthorizationDecision;
use crate::config::security_builder::SecurityBuilder;
use crate::config::web::abstract_request_matcher_registry::AbstractRequestMatcherRegistry;
use crate::config::web::http_security_builder::HttpSecurityBuilder;
use crate::web::access::intercept::authorization_filter::AuthorizationFilter;
use crate::web::default_security_filter_chain::DefaultSecurityFilterChain;
use crate::web::util::matcher::request_matcher_entry::RequestMatcherEntry;
use crate::{
    access::intercept::request_authorization_context::RequestAuthorizationContext,
    authorization::{
        authenticated_authorization_manager::AuthenticatedAuthorizationManager,
        authority_authorization_manager::AuthorityAuthorizationManager,
        authorization_event_publisher::AuthorizationEventPublisher,
        authorization_manager::{AuthorizationManager, DefaultAuthorizationManager},
    },
    config::{
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::util::matcher::{
            ant_path_request_matcher::AntPathRequestMatcher,
            any_request_matcher::AnyRequestMatcher, request_matcher::RequestMatcher,
        },
    },
    web::access::intercept::request_matcher_delegating_authorization_manager::RequestMatcherDelegatingAuthorizationManagerBuilder,
};

const ROLE_PREFIX: &str = "ROLE_";

#[derive(Clone)]
struct NullAuthorizationEventPublisher;

impl AuthorizationEventPublisher for NullAuthorizationEventPublisher {
    fn publish_authorization_event(
        &self,
        _authentication: std::sync::Arc<dyn crate::core::authentication::Authentication>,
        _object_description: &str,
        _result: Option<std::sync::Arc<dyn crate::authorization::authorization_result::AuthorizationResult>>,
    ) {
    }
}

#[derive(Clone)]
struct NullRoleHierarchy;

impl RoleHierarchy for NullRoleHierarchy {
    fn get_reachable_granted_authorities(&self, authorities: &[String]) -> Vec<String> {
        authorities.to_vec()
    }
}

#[derive(Clone)]
pub struct AuthorizeHttpRequestsConfigurer<H>
where
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    _marker: PhantomData<H>,
    registry: AuthorizationManagerRequestMatcherRegistry,
    publisher: Arc<dyn AuthorizationEventPublisher>,
    role_hierarchy: Arc<dyn Fn() -> Arc<dyn RoleHierarchy> + Send + Sync>,
    security_configurer_adapter: SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>,
}

impl<H> AuthorizeHttpRequestsConfigurer<H>
where
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    pub fn open(&self) {}

    pub fn new(_context: &ApplicationContext) -> Self {
        Self {
            _marker: PhantomData,
            registry: AuthorizationManagerRequestMatcherRegistry::default(),
            publisher: Arc::new(NullAuthorizationEventPublisher),
            role_hierarchy: Arc::new(|| Arc::new(NullRoleHierarchy)),
            security_configurer_adapter: SecurityConfigurerAdapter::default(),
        }
    }

    pub fn get_registry(&self) -> AuthorizationManagerRequestMatcherRegistry {
        self.registry.clone()
    }

    pub fn permit_all_authorization_manager() -> AuthorizationDecision {
        AuthorizationDecision::new(true)
    }
}

impl<H> SecurityConfigurer<AuthorizeHttpRequestsConfigurer<H>, H>
    for AuthorizeHttpRequestsConfigurer<H>
where
    H: Send + Sync,
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
    H: SecurityBuilder<AuthorizeHttpRequestsConfigurer<H>>,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, _http: &mut H) {
        let _authorization_filter =
            AuthorizationFilter::new(self.registry.create_authorization_manager());
        let _publisher = self.publisher.clone();
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for AuthorizeHttpRequestsConfigurer<H>
where
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        &self.security_configurer_adapter
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        &mut self.security_configurer_adapter
    }
}

#[derive(Clone)]
struct RegistryState {
    any_request_configured: bool,
    manager_builder: RequestMatcherDelegatingAuthorizationManagerBuilder,
    unmapped_matchers: Option<Vec<Box<dyn RequestMatcher>>>,
    mapping_count: u32,
    should_filter_all_dispatcher_types: bool,
}

impl Default for RegistryState {
    fn default() -> Self {
        Self {
            any_request_configured: false,
            manager_builder: RequestMatcherDelegatingAuthorizationManagerBuilder::default(),
            unmapped_matchers: None,
            mapping_count: 0,
            should_filter_all_dispatcher_types: true,
        }
    }
}

#[derive(Clone)]
pub struct AuthorizationManagerRequestMatcherRegistry<C = AuthorizedUrl> {
    state: Arc<Mutex<RegistryState>>,
    abstract_request_matcher_registry: AbstractRequestMatcherRegistry<C>,
    role_hierarchy: Arc<dyn RoleHierarchy>,
    _marker: PhantomData<C>,
}

impl Default for AuthorizationManagerRequestMatcherRegistry<AuthorizedUrl> {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(RegistryState::default())),
            abstract_request_matcher_registry: AbstractRequestMatcherRegistry {
                _marker: PhantomData,
            },
            role_hierarchy: Arc::new(NullRoleHierarchy),
            _marker: PhantomData,
        }
    }
}

impl AuthorizationManagerRequestMatcherRegistry<AuthorizedUrl> {
    pub fn any_request(&mut self) -> AuthorizedUrl {
        {
            let state = self.state.lock().expect("authorization registry lock poisoned");
            assert!(
                !state.any_request_configured,
                "Can't configure anyRequest after itself"
            );
        }

        let configurer = self.request_matchers(AnyRequestMatcher);
        self.state
            .lock()
            .expect("authorization registry lock poisoned")
            .any_request_configured = true;
        configurer
    }

    pub fn request_matchers(&mut self, matcher: impl RequestMatcher + Any) -> AuthorizedUrl {
        {
            let state = self.state.lock().expect("authorization registry lock poisoned");
            assert!(
                !state.any_request_configured,
                "Can't configure requestMatchers after anyRequest"
            );
        }

        let mut matchers: Vec<Box<dyn RequestMatcher>> = Vec::new();
        let any: &dyn Any = &matcher;

        let (http_method, patterns) = if let Some(http_method) = any.downcast_ref::<HttpMethod>() {
            (Some(*http_method), vec![])
        } else if let Some((http_method, patterns)) =
            any.downcast_ref::<(HttpMethod, Vec<&'static str>)>()
        {
            (Some(*http_method), patterns.to_owned())
        } else if let Some(patterns) = any.downcast_ref::<Vec<&'static str>>() {
            (None, patterns.to_owned())
        } else if any.downcast_ref::<AnyRequestMatcher>().is_some() {
            (None, vec!["/**"])
        } else {
            (None, vec![])
        };

        for pattern in patterns {
            matchers.push(Box::new(AntPathRequestMatcher::from((http_method, pattern))));
        }

        self.chain_request_matchers(matchers)
    }

    fn add_mapping(
        &mut self,
        matcher: Box<dyn RequestMatcher>,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) {
        let mut state = self.state.lock().expect("authorization registry lock poisoned");
        state.unmapped_matchers = None;
        state.manager_builder.add(matcher, manager);
        state.mapping_count += 1;
    }

    pub fn add_first(
        &mut self,
        matcher: Box<dyn RequestMatcher>,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) {
        let mut state = self.state.lock().expect("authorization registry lock poisoned");
        state.unmapped_matchers = None;
        state
            .manager_builder
            .mappings
            .insert(0, RequestMatcherEntry::new(matcher, manager));
        state.mapping_count += 1;
    }

    pub fn create_authorization_manager(
        &self,
    ) -> Arc<dyn AuthorizationManager<RequestAuthorizationContext>> {
        let state = self.state.lock().expect("authorization registry lock poisoned");
        assert!(
            state.unmapped_matchers.is_none(),
            "An incomplete mapping was found"
        );
        assert!(
            state.mapping_count > 0,
            "At least one mapping is required (for example, authorizeHttpRequests().anyRequest().authenticated())"
        );
        state.manager_builder.build()
    }

    pub fn chain_request_matchers(
        &mut self,
        request_matchers: Vec<Box<dyn RequestMatcher>>,
    ) -> AuthorizedUrl {
        self.state
            .lock()
            .expect("authorization registry lock poisoned")
            .unmapped_matchers = Some(request_matchers.clone());

        AuthorizedUrl {
            matchers: request_matchers,
            ref_matcher_registry: self.clone(),
            role_hierarchy: self.role_hierarchy.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AuthorizedUrl {
    matchers: Vec<Box<dyn RequestMatcher>>,
    ref_matcher_registry: AuthorizationManagerRequestMatcherRegistry,
    role_hierarchy: Arc<dyn RoleHierarchy>,
}

impl AuthorizedUrl {
    pub fn permit_all(&mut self) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(DefaultAuthorizationManager(true))
    }

    pub fn deny_all(&mut self) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(DefaultAuthorizationManager(false))
    }

    pub fn has_role(&mut self, role: &str) -> &mut AuthorizationManagerRequestMatcherRegistry {
        let manager = self.with_role_hierarchy(AuthorityAuthorizationManager::has_any_role(
            ROLE_PREFIX,
            [role.to_string()],
        ));
        self.access(manager)
    }

    pub fn has_any_role(
        &mut self,
        roles: impl IntoIterator<Item = &'static str>,
    ) -> &mut AuthorizationManagerRequestMatcherRegistry {
        let manager = self.with_role_hierarchy(AuthorityAuthorizationManager::has_any_role(
            ROLE_PREFIX,
            roles.into_iter().map(ToString::to_string),
        ));
        self.access(manager)
    }

    pub fn has_authority(
        &mut self,
        authority: &str,
    ) -> &mut AuthorizationManagerRequestMatcherRegistry {
        let manager =
            self.with_role_hierarchy(AuthorityAuthorizationManager::has_authority(authority));
        self.access(manager)
    }

    pub fn has_any_authority(
        &mut self,
        authorities: impl IntoIterator<Item = &'static str>,
    ) -> &mut AuthorizationManagerRequestMatcherRegistry {
        let manager = self.with_role_hierarchy(AuthorityAuthorizationManager::has_any_authority(
            authorities.into_iter().map(ToString::to_string).collect(),
        ));
        self.access(manager)
    }

    pub fn authenticated(&mut self) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(AuthenticatedAuthorizationManager::authenticated())
    }

    pub fn fully_authenticated(&mut self) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(AuthenticatedAuthorizationManager::fully_authenticated())
    }

    pub fn remember_me(&mut self) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(AuthenticatedAuthorizationManager::remember_me())
    }

    pub fn anonymous(&mut self) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(AuthenticatedAuthorizationManager::anonymous())
    }

    pub fn access<T>(&mut self, manager: T) -> &mut AuthorizationManagerRequestMatcherRegistry
    where
        T: AuthorizationManager<RequestAuthorizationContext> + 'static,
    {
        let manager = Arc::new(manager);
        let matchers = std::mem::take(&mut self.matchers);
        for matcher in matchers {
            self.ref_matcher_registry
                .add_mapping(matcher, manager.clone());
        }
        &mut self.ref_matcher_registry
    }

    fn with_role_hierarchy(
        &mut self,
        mut manager: AuthorityAuthorizationManager<RequestAuthorizationContext>,
    ) -> AuthorityAuthorizationManager<RequestAuthorizationContext> {
        manager.set_role_hierarchy(self.role_hierarchy.clone());
        manager
    }
}

pub struct RequestMatchers;

impl RequestMatchers {
    pub fn ant_matchers_as_array() -> Vec<Box<dyn RequestMatcher>> {
        vec![]
    }
}
