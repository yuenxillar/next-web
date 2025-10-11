use std::{any::Any, cell::RefCell, marker::PhantomData, sync::Arc};

use axum::extract::Request;
use next_web_core::traits::Required::Required;
use next_web_core::util::http_method::HttpMethod;
use next_web_core::ApplicationContext;

use crate::access::hierarchicalroles::role_hierarchy::RoleHierarchy;
use crate::authorization::authorization_decision::AuthorizationDecision;
use crate::config::web::abstract_request_matcher_registry::AbstractRequestMatcherRegistry;
use crate::config::web::http_security_builder::HttpSecurityBuilder;
use crate::web::access::intercept::authorization_filter::AuthorizationFilter;
use crate::web::access::intercept::request_matcher_delegating_authorization_manager::RequestMatcherDelegatingAuthorizationManager;
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

pub struct AuthorizeHttpRequestsConfigurer<H> {
    _marker: PhantomData<H>,
    registry: AuthorizationManagerRequestMatcherRegistry,
    publisher: Arc<dyn AuthorizationEventPublisher>,
    role_hierarchy: Arc<dyn Fn() -> Arc<dyn RoleHierarchy>>,

    security_configurer_adapter: SecurityConfigurerAdapter<()>,
}

impl<H> AuthorizeHttpRequestsConfigurer<H> {
    pub fn open(&self) -> () {}

    pub fn new(
        context: &ApplicationContext
    ) -> Self {
        Self {
            _marker: (),
            registry: (),
            publisher: (),
            role_hierarchy: (),
            security_configurer_adapter: (),
        }
    }


    pub fn get_registry(&self) -> AuthorizationManagerRequestMatcherRegistry {
        self.registry.clone()
    }
    fn add_mapping<T1, T2>(
        &mut self,
        matchers: T1,
        manager: T2,
    ) -> &AuthorizationManagerRequestMatcherRegistry
    where
        T1: IntoIterator<Item = Box<dyn RequestMatcher>>,
        T2: AuthorizationManager<RequestAuthorizationContext> + 'static,
    {
        let manager = Arc::new(manager);
        for matcher in matchers {
            self.registry.add_mapping(matcher, manager.to_owned());
        }

        &self.registry
    }

    pub fn permit_all_authorization_manager() -> AuthorizationDecision {
        AuthorizationDecision::new(true)
    }
}

impl<H> SecurityConfigurer<H> for AuthorizeHttpRequestsConfigurer<H>
where
    H: Send + Sync,
    H: HttpSecurityBuilder<H>,
{
    fn init(&self, _http: H) {}

    fn configure(&self, http: H) {
        let authorization_manager = self.registry.create_authorization_manager();
        let authorization_filter = AuthorizationFilter::new(authorization_manager);
        // TODO
        // authorization_filter.setAuthorizationEventPublisher(this.publisher);

        authorization_filter
            .setShouldFilterAllDispatcherTypes(this.registry.shouldFilterAllDispatcherTypes);
        authorization_filter.setSecurityContextHolderStrategy(getSecurityContextHolderStrategy());

        http.add_filter((self.security_configurer_adapter.post_process(object)));
    }
}


impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>> for  AuthorizeHttpRequestsConfigurer<H> {
    fn get_object(&self) -> & SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        todo!()
    }

    fn get_object_mut(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        todo!()
    }
}

#[derive(Clone)]
pub struct AuthorizationManagerRequestMatcherRegistry<C = AuthorizedUrl> {
    any_request_configured: bool,

    manager_builder: RequestMatcherDelegatingAuthorizationManagerBuilder,
    unmapped_matchers: Option<Vec<Box<dyn RequestMatcher>>>,
    mapping_count: u32,
    should_filter_all_dispatcher_types: bool,

    abstract_request_matcher_registry: AbstractRequestMatcherRegistry<C>,
    _marker: PhantomData<C>,
}

impl AuthorizationManagerRequestMatcherRegistry<AuthorizedUrl> {
    pub fn any_request(&mut self) -> AuthorizedUrl {
        assert!(
            !self.any_request_configured,
            "Can't configure anyRequest after itself"
        );
        let configurer = self.request_matchers(AnyRequestMatcher::default());

        self.any_request_configured = true;
        configurer
    }

    pub fn request_matchers(&mut self, matcher: impl RequestMatcher + Any) -> AuthorizedUrl {
        assert!(
            !self.any_request_configured,
            "Can't configure requestMatchers after anyRequest"
        );
        let mut matchers: Vec<Box<dyn RequestMatcher>> = Vec::new();
        let any: &dyn Any = &matcher;

        let (http_method, pattens) = if let Some(http_method) = any.downcast_ref::<HttpMethod>() {
            (Some(*http_method), vec![])
        } else if let Some((http_method, pattens)) =
            any.downcast_ref::<(HttpMethod, Vec<&'static str>)>()
        {
            (Some(*http_method), pattens.to_owned())
        } else if let Some(pattens) = any.downcast_ref::<Vec<&'static str>>() {
            (None, pattens.to_owned())
        } else if let Some(_matcher) = any.downcast_ref::<AnyRequestMatcher>() {
            (None, vec!["/**"])
        } else {
            (None, vec![])
        };

        for pattern in pattens {
            let ant = AntPathRequestMatcher::from((http_method, pattern));
            matchers.push(Box::new(ant));
        }

        self.chain_request_matchers(matchers)
    }

    fn add_mapping(
        &mut self,
        matcher: Box<dyn RequestMatcher>,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) {
        self.unmapped_matchers = None;
        self.manager_builder.add(matcher, manager);
        self.mapping_count += 1;
    }

    fn add_first(
        &mut self,
        matcher: Box<dyn RequestMatcher>,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) {
        self.unmapped_matchers = None;
        self.manager_builder
            .mappings
            .insert(0, RequestMatcherEntry::new(matcher, manager));
        self.mapping_count += 1;
    }

    fn create_authorization_manager(&self) -> Arc<dyn AuthorizationManager<Request>> {
        assert!(self.unmapped_matchers.is_none(), "An incomplete mapping was found for [{:?}] . Try completing it with something like requestUrls().<something>.hasRole('USER')",
        self.unmapped_matchers);

        assert!(self.mapping_count > 0, "At least one mapping is required (for example, authorizeHttpRequests().anyRequest().authenticated())");

        // self.post_process(self.manager_builder.build())
        todo!()
    }
}

impl AuthorizationManagerRequestMatcherRegistry<AuthorizedUrl> {
    pub fn chain_request_matchers(
        &mut self,
        request_matchers: Vec<Box<dyn RequestMatcher>>,
    ) -> AuthorizedUrl {
        self.unmapped_matchers = Some(request_matchers.clone());

        AuthorizedUrl {
            matchers: request_matchers,
            ref_matcher_registry: todo!(),
            role_hierarchy: todo!(),
        }
    }
}
pub struct AuthorizedUrl {
    matchers: Vec<Box<dyn RequestMatcher>>,

    ref_matcher_registry: RefCell<AuthorizationManagerRequestMatcherRegistry>,

    role_hierarchy: Box<dyn Fn() -> Arc<dyn RoleHierarchy>>,
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

        let iters = std::mem::replace(&mut self.matchers, vec![]);
        for matcher in iters {
            self.ref_matcher_registry
                .get_mut()
                .add_mapping(matcher, manager.to_owned());
        }

        self.ref_matcher_registry.get_mut()
    }

    fn with_role_hierarchy(
        &mut self,
        mut manager: AuthorityAuthorizationManager<RequestAuthorizationContext>,
    ) -> AuthorityAuthorizationManager<RequestAuthorizationContext> {
        manager.set_role_hierarchy((self.role_hierarchy)());
        manager
    }
}

pub struct RequestMatchers;

impl RequestMatchers {
    pub fn ant_matchers_as_array() -> Vec<Box<dyn RequestMatcher>> {
        vec![]
    }
}
