use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use next_web_context::ApplicationEventPublisher;
use next_web_core::ApplicationContext;

use crate::{
    access::hierarchicalroles::NullRoleHierarchy, web::access::intercept::AuthorizationFilter,
};
use crate::{
    access::hierarchicalroles::RoleHierarchy,
    config::core::granted_authority_defaults::GrantedAuthorityDefaults,
    web::util::matcher::{Builder, MatcherInput},
};
use crate::{
    access::intercept::RequestAuthorizationContext,
    authorization::{AuthorizationEventPublisher, AuthorizationManager},
    config::security_configurer::SecurityConfigurer,
    web::access::intercept::RequestMatcherDelegatingAuthorizationManagerBuilder,
    web::util::matcher::RequestMatcher,
};
use crate::{authorization::AuthorizationManagerFactory, config::web::BaseRequestMatcherRegistry};
use crate::{
    authorization::AuthorizationManagers, config::web::http_security_builder::HttpSecurityBuilder,
};
use crate::{
    authorization::DefaultAuthorizationManagerFactory, config::web::configurers::BaseHttpConfigurer,
};
use crate::{
    authorization::NextAuthorizationEventPublisher,
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};
use crate::{config::web::BaseRequestMatcherRegistryExt, web::util::matcher::RequestMatcherEntry};

/// Adds a URL based authorization using AuthorizationManager.
#[derive(Clone)]
pub struct AuthorizeHttpRequestsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    registry: AuthorizationManagerRequestMatcherRegistry,
    publisher: Arc<dyn AuthorizationEventPublisher>,
    authorization_manager_factory:
        Arc<dyn AuthorizationManagerFactory<RequestAuthorizationContext>>,

    inner: BaseHttpConfigurer<Self, H>,
}

impl<H> AuthorizeHttpRequestsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Creates an instance.
    pub fn new(ctx: &mut ApplicationContext) -> Self {
        let authorization_manager_factory = Self::get_authorization_manager_factory(ctx);
        let mut registry =
            AuthorizationManagerRequestMatcherRegistry::with_authorization_manager_factory(
                authorization_manager_factory.clone(),
            );
        registry.inner.set_request_matcher_builder(
            ctx.get_single_option::<Builder>()
                .map(Clone::clone)
                .unwrap_or_default(),
        );
        let publisher = Self::get_authorization_event_publisher(ctx);

        Self {
            registry,
            publisher,
            authorization_manager_factory,

            inner: Default::default(),
        }
    }

    fn get_authorization_event_publisher(
        ctx: &mut ApplicationContext,
    ) -> Arc<dyn AuthorizationEventPublisher> {
        if let Some(publisher) = ctx
            .resolve_by_type::<Arc<dyn AuthorizationEventPublisher>>()
            .into_iter()
            .next()
        {
            return publisher;
        }

        ctx.resolve_by_type::<Arc<dyn ApplicationEventPublisher>>()
            .into_iter()
            .next()
            .map(|publisher| {
                Arc::new(NextAuthorizationEventPublisher::new(publisher))
                    as Arc<dyn AuthorizationEventPublisher>
            })
            .expect("No ApplicationEventPublisher found")
    }

    fn get_authorization_manager_factory(
        ctx: &mut ApplicationContext,
    ) -> Arc<dyn AuthorizationManagerFactory<RequestAuthorizationContext>> {
        get_authorization_manager_factory(ctx)
    }

    pub fn registry(&self) -> &AuthorizationManagerRequestMatcherRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut AuthorizationManagerRequestMatcherRegistry {
        &mut self.registry
    }

    pub fn add_mapping(
        &mut self,
        matcher: Vec<Arc<dyn RequestMatcher>>,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) -> &mut AuthorizationManagerRequestMatcherRegistry {
        for matcher in matcher.into_iter() {
            self.registry.add_mapping(matcher, manager.to_owned());
        }

        &mut self.registry
    }

    pub fn add_first(
        &mut self,
        matcher: Arc<dyn RequestMatcher>,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.registry.add_first(matcher, manager);

        &mut self.registry
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for AuthorizeHttpRequestsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, http: &mut H) {
        let authorization_manager = self.registry.create_authorization_manager();

        let mut authorization_filter = AuthorizationFilter::new(authorization_manager);
        authorization_filter.set_authorization_event_publisher(self.publisher.to_owned());
        authorization_filter.set_security_context_holder_strategy(
            self.inner.get_security_context_holder_strategy().to_owned(),
        );

        http.add_filter(authorization_filter);
    }
}

#[derive(Clone)]
struct RegistryState {
    any_request_configured: bool,
    manager_builder: RequestMatcherDelegatingAuthorizationManagerBuilder,
    unmapped_matchers: Option<Vec<Arc<dyn RequestMatcher>>>,
    mapping_count: u32,
}

impl Default for RegistryState {
    fn default() -> Self {
        Self {
            any_request_configured: false,
            manager_builder: RequestMatcherDelegatingAuthorizationManagerBuilder::default(),
            unmapped_matchers: None,
            mapping_count: 0,
        }
    }
}

#[derive(Clone)]
pub struct AuthorizationManagerRequestMatcherRegistry<C = AuthorizedUrl> {
    state: Arc<Mutex<RegistryState>>,
    authorization_manager_factory:
        Arc<dyn AuthorizationManagerFactory<RequestAuthorizationContext>>,

    inner: BaseRequestMatcherRegistry<C>,
}

impl AuthorizationManagerRequestMatcherRegistry<AuthorizedUrl> {
    pub fn new(ctx: &mut ApplicationContext) -> Self {
        Self::with_authorization_manager_factory(get_authorization_manager_factory(ctx))
    }

    pub fn with_authorization_manager_factory(
        authorization_manager_factory: Arc<
            dyn AuthorizationManagerFactory<RequestAuthorizationContext>,
        >,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(RegistryState::default())),
            authorization_manager_factory,

            inner: Default::default(),
        }
    }
}

impl Default for AuthorizationManagerRequestMatcherRegistry<AuthorizedUrl> {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(RegistryState::default())),
            authorization_manager_factory: Arc::new(DefaultAuthorizationManagerFactory::default()),

            inner: Default::default(),
        }
    }
}

impl AuthorizationManagerRequestMatcherRegistry<AuthorizedUrl> {
    fn add_mapping(
        &mut self,
        matcher: Arc<dyn RequestMatcher>,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) {
        add_mapping(matcher, manager, &self.state);
    }

    fn add_first(
        &mut self,
        matcher: Arc<dyn RequestMatcher>,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) {
        let mut state = self
            .state
            .lock()
            .expect("authorization registry lock poisoned");
        state.unmapped_matchers = None;
        state
            .manager_builder
            .mappings
            .insert(0, RequestMatcherEntry::new(matcher, manager));
        state.mapping_count += 1;
    }

    fn create_authorization_manager(
        &self,
    ) -> Arc<dyn AuthorizationManager<RequestAuthorizationContext>> {
        let mut state = self
            .state
            .lock()
            .expect("authorization registry lock poisoned");
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

    pub fn any_request(&mut self) -> AuthorizedUrl {
        {
            let state = self
                .state
                .lock()
                .expect("authorization registry lock poisoned");
            assert!(
                !state.any_request_configured,
                "Can't configure anyRequest after itself"
            );
        }
        self.inner.any_request();
        let matchers = self.inner.take_request_matchers();
        let authorized_url = self.chain_request_matchers(matchers);
        let mut state = self
            .state
            .lock()
            .expect("authorization registry lock poisoned");
        state.any_request_configured = true;
        drop(state);

        authorized_url
    }

    pub fn request_matchers<T>(&mut self, matcher: T) -> AuthorizedUrl
    where
        T: Into<MatcherInput>,
    {
        self.assert_can_add_matchers();
        self.inner.request_matchers(matcher);
        let matchers = self.inner.take_request_matchers();
        self.chain_request_matchers(matchers)
    }

    fn assert_can_add_matchers(&self) {
        let state = self
            .state
            .lock()
            .expect("authorization registry lock poisoned");
        assert!(
            !state.any_request_configured,
            "Can't configure requestMatchers after anyRequest"
        );
    }
}

impl BaseRequestMatcherRegistryExt<AuthorizedUrl>
    for AuthorizationManagerRequestMatcherRegistry<AuthorizedUrl>
{
    fn chain_request_matchers(&mut self, matchers: Vec<Arc<dyn RequestMatcher>>) -> AuthorizedUrl {
        let mut state = self
            .state
            .lock()
            .expect("authorization registry lock poisoned");
        state.unmapped_matchers = Some(matchers.clone());
        drop(state);

        AuthorizedUrl::new(
            matchers,
            self.authorization_manager_factory.clone(),
            self.clone(),
        )
    }
}

impl Deref for AuthorizationManagerRequestMatcherRegistry<AuthorizedUrl> {
    type Target = BaseRequestMatcherRegistry<AuthorizedUrl>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for AuthorizationManagerRequestMatcherRegistry<AuthorizedUrl> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// An object that allows configuring the [`AuthorizationManager`] for
/// [`RequestMatcher`]s.
///
/// This is the fluent API that users interact with after specifying which
/// requests they want to authorize.
#[derive(Clone)]
pub struct AuthorizedUrl {
    matchers: Vec<Arc<dyn RequestMatcher>>,
    authorization_manager_factory:
        Arc<dyn AuthorizationManagerFactory<RequestAuthorizationContext>>,
    not: bool,

    registry: AuthorizationManagerRequestMatcherRegistry,
}

impl AuthorizedUrl {
    /// Creates a new `AuthorizedUrl` instance.
    pub fn new(
        matchers: Vec<Arc<dyn RequestMatcher>>,
        authorization_manager_factory: Arc<
            dyn AuthorizationManagerFactory<RequestAuthorizationContext>,
        >,
        registry: AuthorizationManagerRequestMatcherRegistry,
    ) -> Self {
        Self {
            matchers,
            authorization_manager_factory,
            not: false,

            registry,
        }
    }

    pub fn get_matchers(&self) -> &[Arc<dyn RequestMatcher>] {
        &self.matchers
    }

    pub fn set_authorization_manager_factory(
        &mut self,
        factory: Arc<dyn AuthorizationManagerFactory<RequestAuthorizationContext>>,
    ) {
        self.authorization_manager_factory = factory;
    }

    /// Negates the following authorization rule.
    pub fn not(mut self) -> Self {
        self.not = true;
        self
    }

    /// Specifies that URLs are allowed by anyone.
    pub fn permit_all(&mut self) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(self.authorization_manager_factory.permit_all())
    }

    /// Specifies that URLs are not allowed by anyone.
    pub fn deny_all(&mut self) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(self.authorization_manager_factory.deny_all())
    }

    /// Specifies that a user requires a role.
    ///
    /// The role is automatically prepended with "ROLE_" (configurable).
    pub fn has_role(&mut self, role: &str) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(self.authorization_manager_factory.has_role(role))
    }

    /// Specifies that a user requires one of many roles.
    pub fn has_any_role(
        &mut self,
        roles: &[&str],
    ) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(
            self.authorization_manager_factory
                .has_any_role(&roles.iter().map(ToString::to_string).collect::<Vec<_>>()),
        )
    }

    /// Specifies that a user requires all the provided roles.
    pub fn has_all_roles(
        &mut self,
        roles: &[&str],
    ) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(
            self.authorization_manager_factory
                .has_all_roles(&roles.iter().map(ToString::to_string).collect::<Vec<_>>()),
        )
    }

    /// Specifies that a user requires an authority.
    pub fn has_authority(
        &mut self,
        authority: &str,
    ) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(self.authorization_manager_factory.has_authority(authority))
    }

    /// Specifies that a user requires one of many authorities.
    pub fn has_any_authority(
        &mut self,
        authorities: &[&str],
    ) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(
            self.authorization_manager_factory.has_any_authority(
                &authorities
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            ),
        )
    }

    /// Specifies that a user requires all the provided authorities.
    pub fn has_all_authorities(
        &mut self,
        authorities: &[&str],
    ) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(
            self.authorization_manager_factory.has_all_authorities(
                &authorities
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            ),
        )
    }

    /// Specifies that URLs are allowed by any authenticated user.
    pub fn authenticated(&mut self) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(self.authorization_manager_factory.authenticated())
    }

    /// Specifies that URLs are allowed by users who have authenticated and were not "remembered".
    pub fn fully_authenticated(&mut self) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(self.authorization_manager_factory.fully_authenticated())
    }

    /// Specifies that URLs are allowed by users that have been remembered.
    pub fn remember_me(&mut self) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(self.authorization_manager_factory.remember_me())
    }

    /// Specifies that URLs are allowed by anonymous users.
    pub fn anonymous(&mut self) -> &mut AuthorizationManagerRequestMatcherRegistry {
        self.access(self.authorization_manager_factory.anonymous())
    }

    /// Specify that a path variable in URL to be compared.
    pub fn has_variable<'a>(&mut self, variable: &'a str) -> AuthorizedUrlVariable<'a> {
        AuthorizedUrlVariable::new(variable)
    }

    /// Allows specifying a custom [`AuthorizationManager`].
    pub fn access(
        &mut self,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) -> &mut AuthorizationManagerRequestMatcherRegistry {
        let state = &self.registry.state;
        if self.not {
            let not_manager = Arc::new(AuthorizationManagers::not(manager));
            for matcher in self.matchers.iter() {
                add_mapping(matcher.clone(), not_manager.clone(), state);
            }
        } else {
            for matcher in self.matchers.iter() {
                add_mapping(matcher.clone(), manager.clone(), state);
            }
        }

        &mut self.registry
    }
}

/// An object that allows configuring RequestMatchers with URI path variables
pub struct AuthorizedUrlVariable<'a> {
    /// The name of the path variable to compare.
    variable: &'a str,
}

impl<'a> AuthorizedUrlVariable<'a> {
    /// Creates a new `AuthorizedUrlVariable` instance.
    ///
    /// This is typically called by [`AuthorizedUrl::has_variable`].
    pub fn new(variable: &'a str) -> Self {
        Self { variable }
    }

    // /// Compares the value of a path variable in the URI with an `Authentication` attribute.
    // ///
    // /// This method takes a function that extracts a value from the `Authentication`
    // /// object and compares it with the path variable value extracted from the
    // /// request context.
    // ///
    // /// # Type Parameters
    // /// * `F` - The function type that maps `Authentication` to `String`.
    // ///
    // /// # Arguments
    // /// * `function` - A function that extracts a `String` value from `Authentication`.
    // ///
    // /// # Returns
    // /// Returns a mutable reference to the parent [`AuthorizationManagerRequestMatcherRegistry`]
    // /// for further customization.
    // ///
    // /// # Example
    // /// ```rust
    // /// # use authorize_http_requests_configurer::*;
    // /// registry
    // ///     .request_matchers(&["/user/{username}"])
    // ///     .has_variable("username")
    // ///     .equal_to(|auth| auth.name().to_string());
    // /// ```
    // pub fn equal_to<'b, F>(
    //     &'a mut self,
    //     function: F,
    // ) -> &'a mut AuthorizationManagerRequestMatcherRegistry
    // where
    //     F: Fn(&'b dyn crate::core::Authentication) -> &'b str,
    // {
    //     // let value = function();
    //     // let value1 = "";
    //     // // AuthorizationDecision::new(value1 == value)
    //     // self.registry
    // }
}

fn add_mapping(
    matcher: Arc<dyn RequestMatcher>,
    manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    state: &Arc<Mutex<RegistryState>>,
) {
    let mut state = state.lock().expect("authorization registry lock poisoned");
    state.unmapped_matchers = None;
    state.manager_builder.add(matcher, manager);
    state.mapping_count += 1;
}

fn get_authorization_manager_factory(
    ctx: &mut ApplicationContext,
) -> Arc<dyn AuthorizationManagerFactory<RequestAuthorizationContext>> {
    let factories =
        ctx.resolve_by_type::<Arc<dyn AuthorizationManagerFactory<RequestAuthorizationContext>>>();
    factories.into_iter().next().unwrap_or_else(|| {
        let role_hierarchy = ctx
            .resolve_by_type::<Arc<dyn RoleHierarchy>>()
            .into_iter()
            .next()
            .unwrap_or(Arc::new(NullRoleHierarchy::default()));
        let granted_authority_defaults = ctx.resolve_option::<GrantedAuthorityDefaults>();

        let role_prefix = granted_authority_defaults
            .as_ref()
            .map(|s| s.role_prefix())
            .unwrap_or("ROLE_");

        let mut authorization_manager_factory = DefaultAuthorizationManagerFactory::default();
        authorization_manager_factory.set_role_hierarchy(role_hierarchy);
        authorization_manager_factory.set_role_prefix(role_prefix);

        Arc::new(authorization_manager_factory)
    })
}

impl<H> Deref for AuthorizeHttpRequestsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for AuthorizeHttpRequestsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
