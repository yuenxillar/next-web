use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_context::{
    event::GenericApplicationListenerAdapter, ApplicationEvent, ApplicationListener,
};
use next_web_core::{traits::required::Required, ApplicationContext};

use crate::{
    authorization::AuthenticationTrustResolver,
    config::{
        http::SessionCreationPolicy,
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::{BaseHttpConfigurer, LogoutConfigurer},
            http_security_builder::HttpSecurityBuilder,
        },
    },
    context::DelegatingApplicationListener,
    core::session::{SessionRegistry, SessionRegistryImpl},
    web::{
        authentication::{
            session::{
                session_limit_of, ChangeSessionIdAuthenticationStrategy,
                CompositeSessionAuthenticationStrategy,
                ConcurrentSessionControlAuthenticationStrategy,
                RegisterSessionAuthenticationStrategy, SessionAuthenticationStrategy,
                SessionFixationProtectionStrategy, SessionLimit,
            },
            AuthenticationFailureHandler, SimpleUrlAuthenticationFailureHandler,
        },
        context::{
            DelegatingSecurityContextRepository, HttpSessionSecurityContextRepository,
            NullSecurityContextRepository, RequestAttributeSecurityContextRepository,
            SecurityContextRepository,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
        savedrequest::{NullRequestCache, RequestCache},
        session::{
            ConcurrentSessionFilter, ForceEagerSessionCreationFilter, InvalidSessionStrategy,
            SessionInformationExpiredStrategy, SessionManagementFilter,
            SimpleRedirectInvalidSessionStrategy, SimpleRedirectSessionInformationExpiredStrategy,
        },
    },
};

/// Allows configuring session management.
///
/// # Security Filters
///
/// The following Filters are populated:
///
/// * `SessionManagementFilter`
/// * `ConcurrentSessionFilter` if there are restrictions on how many concurrent sessions
///   a user can have
///
/// # Shared Objects Created
///
/// The following shared objects are created:
///
/// * `RequestCache`
/// * `SecurityContextRepository`
/// * `SessionManagementConfigurer`
/// * `InvalidSessionStrategy`
///
/// # Shared Objects Used
///
/// * `SecurityContextRepository`
/// * `AuthenticationTrustResolver` is optionally used to populate the
///   `HttpSessionSecurityContextRepository` and `SessionManagementFilter`
#[derive(Clone)]
pub struct SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    default_session_fixation_strategy: Arc<dyn SessionAuthenticationStrategy>,
    pub(super) session_fixation_authentication_strategy:
        Option<Arc<dyn SessionAuthenticationStrategy>>,
    session_authentication_strategy: Option<Arc<dyn SessionAuthenticationStrategy>>,
    provided_session_authentication_strategy: Option<Arc<dyn SessionAuthenticationStrategy>>,
    invalid_session_strategy: Option<Arc<dyn InvalidSessionStrategy>>,
    expired_session_strategy: Option<Arc<dyn SessionInformationExpiredStrategy>>,
    session_authentication_strategies: Vec<Arc<dyn SessionAuthenticationStrategy>>,
    session_registry: Option<Arc<dyn SessionRegistry>>,
    session_limit: Option<SessionLimit>,
    expired_url: Option<String>,
    max_sessions_prevents_login: bool,
    session_policy: Option<SessionCreationPolicy>,
    enable_session_url_rewriting: bool,
    invalid_session_url: Option<String>,
    session_authentication_error_url: Option<String>,
    session_authentication_failure_handler: Option<Arc<dyn AuthenticationFailureHandler>>,
    properties_that_require_implicit_authentication: HashSet<String>,
    require_explicit_authentication_strategy: Option<bool>,
    /// This should not use RequestAttributeSecurityContextRepository since that is
    /// stateless and session management is about state management.
    session_management_security_context_repository: Option<Arc<dyn SecurityContextRepository>>,

    base: BaseHttpConfigurer<Self, H>,
}

impl<H> SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Setting this attribute will inject the `SessionManagementFilter` with a
    /// `SimpleRedirectInvalidSessionStrategy` configured with the attribute value. When
    /// an invalid session ID is submitted, the strategy will be invoked, redirecting to
    /// the configured URL.
    ///
    /// # Arguments
    ///
    /// * `invalid_session_url` - The URL to redirect to when an invalid session is
    ///   detected.
    pub fn invalid_session_url(&mut self, invalid_session_url: &str) -> &mut Self {
        self.invalid_session_url = Some(invalid_session_url.to_string());
        self.properties_that_require_implicit_authentication
            .insert(format!("invalid_session_url = {}", invalid_session_url));
        self
    }

    /// Setting this means that explicit invocation of `SessionAuthenticationStrategy` is
    /// required.
    ///
    /// # Arguments
    ///
    /// * `require_explicit_authentication_strategy` - Whether explicit invocation of
    ///   `SessionAuthenticationStrategy` is required.
    pub fn require_explicit_authentication_strategy(
        &mut self,
        require_explicit_authentication_strategy: bool,
    ) -> &mut Self {
        self.require_explicit_authentication_strategy =
            Some(require_explicit_authentication_strategy);
        self
    }

    /// Setting this attribute will inject the provided `InvalidSessionStrategy` into the
    /// `SessionManagementFilter`. When an invalid session ID is submitted, the strategy
    /// will be invoked, redirecting to the configured URL.
    ///
    /// # Arguments
    ///
    /// * `invalid_session_strategy` - The strategy to use when an invalid session ID is
    ///   submitted.
    pub fn invalid_session_strategy(
        &mut self,
        invalid_session_strategy: Arc<dyn InvalidSessionStrategy>,
    ) -> &mut Self {
        self.properties_that_require_implicit_authentication
            .insert(format!(
                "invalid_session_strategy = {:?}",
                invalid_session_strategy
            ));
        self.invalid_session_strategy = Some(invalid_session_strategy);
        self
    }

    /// Defines the URL of the error page which should be shown when the
    /// `SessionAuthenticationStrategy` raises an exception. If not set, an unauthorized
    /// (402) error code will be returned to the client. Note that this attribute doesn't
    /// apply if the error occurs during a form-based login, where the URL for
    /// authentication failure will take precedence.
    ///
    /// # Arguments
    ///
    /// * `session_authentication_error_url` - The URL to redirect to.
    pub fn session_authentication_error_url(
        &mut self,
        session_authentication_error_url: &str,
    ) -> &mut Self {
        self.session_authentication_error_url = Some(session_authentication_error_url.to_string());
        self.properties_that_require_implicit_authentication
            .insert(format!(
                "session_authentication_error_url = {}",
                session_authentication_error_url
            ));
        self
    }

    /// Defines the `AuthenticationFailureHandler` which will be used when the
    /// `SessionAuthenticationStrategy` raises an exception. If not set, an unauthorized
    /// (402) error code will be returned to the client. Note that this attribute doesn't
    /// apply if the error occurs during a form-based login, where the URL for
    /// authentication failure will take precedence.
    ///
    /// # Arguments
    ///
    /// * `session_authentication_failure_handler` - The handler to use.
    pub fn session_authentication_failure_handler(
        &mut self,
        session_authentication_failure_handler: Arc<dyn AuthenticationFailureHandler>,
    ) -> &mut Self {
        self.properties_that_require_implicit_authentication
            .insert(format!(
                "session_authentication_failure_handler = {:?}",
                session_authentication_failure_handler.as_ref()
            ));
        self.session_authentication_failure_handler = Some(session_authentication_failure_handler);
        self
    }

    /// If set to true, allows HTTP sessions to be rewritten in the URLs when using
    /// `encode_redirect_url` or `encode_url`, otherwise disallows HTTP sessions to be
    /// included in the URL. This prevents leaking information to external domains.
    ///
    /// This is achieved by guarding `encode_url` and `encode_redirect_url` invocations.
    /// Any code that also overrides either of these two methods needs to come after the
    /// security filter chain or risk being skipped.
    ///
    /// # Arguments
    ///
    /// * `enable_session_url_rewriting` - true if should allow the JSESSIONID to be
    ///   rewritten into the URLs, else false (default).
    pub fn enable_session_url_rewriting(
        &mut self,
        enable_session_url_rewriting: bool,
    ) -> &mut Self {
        self.enable_session_url_rewriting = enable_session_url_rewriting;
        self
    }

    /// Allows specifying the `SessionCreationPolicy`.
    ///
    /// # Arguments
    ///
    /// * `session_creation_policy` - The `SessionCreationPolicy` to use. Cannot be null.
    pub fn session_creation_policy(
        &mut self,
        session_creation_policy: SessionCreationPolicy,
    ) -> &mut Self {
        self.properties_that_require_implicit_authentication
            .insert(format!(
                "session_creation_policy = {:?}",
                session_creation_policy
            ));
        self.session_policy = Some(session_creation_policy);
        self
    }

    /// Allows explicitly specifying the `SessionAuthenticationStrategy`. The default is
    /// to use `ChangeSessionIdAuthenticationStrategy`. If restricting the maximum number
    /// of sessions is configured, then `CompositeSessionAuthenticationStrategy`
    /// delegating to `ConcurrentSessionControlAuthenticationStrategy`, the default OR
    /// supplied `SessionAuthenticationStrategy` and
    /// `RegisterSessionAuthenticationStrategy`.
    ///
    /// NOTE: Supplying a custom `SessionAuthenticationStrategy` will override the default
    /// session fixation strategy.
    ///
    /// # Arguments
    ///
    /// * `session_authentication_strategy` - The strategy to use.
    pub fn session_authentication_strategy(
        &mut self,
        session_authentication_strategy: Arc<dyn SessionAuthenticationStrategy>,
    ) -> &mut Self {
        self.properties_that_require_implicit_authentication
            .insert(format!(
                "session_authentication_strategy = {:?}",
                session_authentication_strategy.as_ref()
            ));
        self.provided_session_authentication_strategy = Some(session_authentication_strategy);
        self
    }

    /// Adds an additional `SessionAuthenticationStrategy` to be used within the
    /// `CompositeSessionAuthenticationStrategy`.
    ///
    /// # Arguments
    ///
    /// * `session_authentication_strategy` - The additional strategy to add.
    pub fn add_session_authentication_strategy(
        &mut self,
        session_authentication_strategy: Arc<dyn SessionAuthenticationStrategy>,
    ) -> &mut Self {
        self.session_authentication_strategies
            .push(session_authentication_strategy);
        self
    }

    /// Allows changing the default `SessionFixationProtectionStrategy`.
    pub fn session_fixation(&mut self) -> SessionFixationConfigurer<'_, H> {
        SessionFixationConfigurer::new(self)
    }

    /// Allows configuring session fixation protection.
    ///
    /// # Arguments
    ///
    /// * `session_fixation_customizer` - The customizer to provide more options for the
    ///   `SessionFixationConfigurer`.
    pub fn session_fixation_with_customizer<F>(
        &mut self,
        session_fixation_customizer: F,
    ) -> &mut Self
    where
        F: FnOnce(&mut SessionFixationConfigurer<H>),
    {
        let mut configurer = SessionFixationConfigurer::new(self);
        session_fixation_customizer(&mut configurer);
        self
    }

    /// Controls the maximum number of sessions for a user. The default is to allow any
    /// number of sessions.
    ///
    /// # Arguments
    ///
    /// * `maximum_sessions` - The maximum number of sessions for a user.
    pub fn maximum_sessions(&mut self, maximum_sessions: i32) -> ConcurrencyControlConfigurer<H> {
        self.session_limit = Some(session_limit_of(maximum_sessions));
        self.properties_that_require_implicit_authentication
            .insert(format!("maximum_sessions = {}", maximum_sessions));
        ConcurrencyControlConfigurer::new(self)
    }

    /// Controls the maximum number of sessions for a user. The default is to allow any
    /// number of users.
    ///
    /// # Arguments
    ///
    /// * `session_concurrency_customizer` - The customizer to provide more options for
    ///   the `ConcurrencyControlConfigurer`.
    pub fn session_concurrency<F>(&mut self, session_concurrency_customizer: F) -> &mut Self
    where
        F: FnOnce(&mut ConcurrencyControlConfigurer<H>),
    {
        let mut configurer = ConcurrencyControlConfigurer::new(self);
        session_concurrency_customizer(&mut configurer);
        self
    }

    /// Invokes `post_process` and sets the `SessionAuthenticationStrategy` for session
    /// fixation.
    fn set_session_fixation_authentication_strategy(
        &mut self,
        session_fixation_authentication_strategy: Arc<dyn SessionAuthenticationStrategy>,
    ) {
        self.session_fixation_authentication_strategy =
            Some(session_fixation_authentication_strategy);
    }

    /// Determines whether explicit authentication strategy is required.
    fn should_require_explicit_authentication_strategy(&self) -> bool {
        let default_require_explicit = self
            .properties_that_require_implicit_authentication
            .is_empty();
        match self.require_explicit_authentication_strategy {
            None => default_require_explicit,
            Some(true) if !default_require_explicit => {
                panic!(
                    "Invalid configuration that explicitly sets \
                        requireExplicitAuthenticationStrategy to true but implicitly \
                        requires it due to the following properties being set: {:?}",
                    self.properties_that_require_implicit_authentication
                );
            }
            Some(value) => value,
        }
    }

    /// Creates the `SessionManagementFilter` if required.
    fn create_session_management_filter(
        &mut self,
        http: &mut H,
    ) -> Option<SessionManagementFilter> {
        if self.should_require_explicit_authentication_strategy() {
            return None;
        }
        let security_context_repository = self
            .session_management_security_context_repository
            .clone()
            .expect("security context repository must be set");
        let mut session_management_filter = SessionManagementFilter::new(
            security_context_repository,
            self.get_session_authentication_strategy(http),
        );

        if let Some(error_url) = self.session_authentication_error_url.as_ref() {
            session_management_filter.set_authentication_failure_handler(Arc::new(
                SimpleUrlAuthenticationFailureHandler::new(error_url),
            ));
        }

        if let Some(strategy) = self.get_invalid_session_strategy() {
            session_management_filter.set_invalid_session_strategy(strategy);
        }

        if let Some(failure_handler) = self.get_session_authentication_failure_handler() {
            session_management_filter.set_authentication_failure_handler(failure_handler);
        }

        let trust_resolver = http.shared_object::<Arc<dyn AuthenticationTrustResolver>>();
        if let Some(trust_resolver) = trust_resolver {
            session_management_filter.set_trust_resolver(trust_resolver.to_owned());
        }

        session_management_filter.set_security_context_holder_strategy(
            self.get_security_context_holder_strategy().to_owned(),
        );

        Some(session_management_filter)
    }

    /// Creates the `ConcurrentSessionFilter` for concurrency control.
    fn create_concurrency_filter(&mut self, http: &mut H) -> ConcurrentSessionFilter
    where
        H: 'static,
    {
        let expire_strategy = self.get_expired_session_strategy();
        let session_registry = self.get_session_registry(http);
        let mut concurrent_session_filter = if let Some(expire_strategy) = expire_strategy {
            ConcurrentSessionFilter::with_expired_strategy(
                session_registry.clone(),
                expire_strategy,
            )
        } else {
            ConcurrentSessionFilter::new(session_registry.clone())
        };

        let logout_configurer = http.configurer_mut::<LogoutConfigurer<H>>();
        if let Some(logout_configurer) = logout_configurer {
            let logout_handlers = logout_configurer.get_logout_handlers();
            if !logout_handlers.is_empty() {
                concurrent_session_filter.set_logout_handlers(logout_handlers.to_vec());
            }
        }

        concurrent_session_filter.set_security_context_holder_strategy(
            self.get_security_context_holder_strategy().to_owned(),
        );

        concurrent_session_filter
    }

    /// Gets the `InvalidSessionStrategy` to use. If null and `invalid_session_url` is
    /// not null, defaults to `SimpleRedirectInvalidSessionStrategy`.
    pub fn get_invalid_session_strategy(&mut self) -> Option<Arc<dyn InvalidSessionStrategy>> {
        if self.invalid_session_strategy.is_some() {
            return self.invalid_session_strategy.clone();
        }
        if let Some(ref invalid_session_url) = self.invalid_session_url {
            let strategy = Arc::new(SimpleRedirectInvalidSessionStrategy::new(
                invalid_session_url,
            ));
            self.invalid_session_strategy = Some(strategy.clone());
            return Some(strategy);
        }
        None
    }

    /// Gets the `SessionInformationExpiredStrategy` to use.
    fn get_expired_session_strategy(
        &mut self,
    ) -> Option<Arc<dyn SessionInformationExpiredStrategy>> {
        if self.expired_session_strategy.is_some() {
            return self.expired_session_strategy.clone();
        }
        if let Some(ref expired_url) = self.expired_url {
            let strategy = Arc::new(SimpleRedirectSessionInformationExpiredStrategy::new(
                expired_url,
            ));
            self.expired_session_strategy = Some(strategy.clone());
            return Some(strategy);
        }
        None
    }

    /// Gets the `AuthenticationFailureHandler` for session authentication errors.
    fn get_session_authentication_failure_handler(
        &mut self,
    ) -> Option<Arc<dyn AuthenticationFailureHandler>> {
        if self.session_authentication_failure_handler.is_some() {
            return self.session_authentication_failure_handler.clone();
        }
        if let Some(ref error_url) = self.session_authentication_error_url {
            let handler = Arc::new(SimpleUrlAuthenticationFailureHandler::new(error_url));
            self.session_authentication_failure_handler = Some(handler.clone());
            return Some(handler);
        }
        None
    }

    /// Gets the `SessionCreationPolicy`. Can not be null.
    fn get_session_creation_policy(&self, http: &H) -> SessionCreationPolicy {
        if let Some(policy) = self.session_policy {
            return policy;
        }
        let session_policy = http
            .shared_object::<SessionCreationPolicy>()
            .map(Clone::clone);
        session_policy.unwrap_or(SessionCreationPolicy::IfRequired)
    }

    /// Returns true if the `SessionCreationPolicy` allows session creation, else false.
    fn is_allow_session_creation(&self, http: &H) -> bool {
        let session_policy = self.get_session_creation_policy(http);
        session_policy == SessionCreationPolicy::Always
            || session_policy == SessionCreationPolicy::IfRequired
    }

    /// Returns true if the `SessionCreationPolicy` is stateless.
    fn is_stateless(&self, http: &H) -> bool {
        self.get_session_creation_policy(http) == SessionCreationPolicy::Stateless
    }

    /// Gets the customized `SessionAuthenticationStrategy` if
    /// `session_authentication_strategy` was specified. Otherwise creates a default
    /// `SessionAuthenticationStrategy`.
    fn get_session_authentication_strategy(
        &mut self,
        http: &mut H,
    ) -> Arc<dyn SessionAuthenticationStrategy> {
        if let Some(ref strategy) = self.session_authentication_strategy {
            return strategy.clone();
        }

        let mut delegate_strategies = self.session_authentication_strategies.clone();
        let default_session_authentication_strategy =
            if let Some(ref provided) = self.provided_session_authentication_strategy {
                Some(provided.clone())
            } else {
                // If the user did not provide a SessionAuthenticationStrategy
                // then default to sessionFixationAuthenticationStrategy
                self.session_fixation_authentication_strategy.clone()
            };

        if self.is_concurrent_session_control_enabled() {
            let session_registry = self.get_session_registry(http);
            let mut concurrent_session_control_strategy =
                ConcurrentSessionControlAuthenticationStrategy::new(session_registry.clone());
            if let Some(ref session_limit) = self.session_limit {
                concurrent_session_control_strategy
                    .set_maximum_sessions_with_limit(session_limit.clone());
            }
            concurrent_session_control_strategy
                .set_error_if_maximum_exceeded(self.max_sessions_prevents_login);
            let register_session_strategy =
                RegisterSessionAuthenticationStrategy::new(session_registry);

            delegate_strategies.push(Arc::new(concurrent_session_control_strategy));
            default_session_authentication_strategy
                .map(|strategy| delegate_strategies.push(strategy));
            delegate_strategies.push(Arc::new(register_session_strategy));
        } else {
            default_session_authentication_strategy
                .map(|strategy| delegate_strategies.push(strategy));
        }

        let strategy = Arc::new(CompositeSessionAuthenticationStrategy::new(
            delegate_strategies,
        ));
        self.session_authentication_strategy = Some(strategy.clone());
        strategy
    }

    /// Gets the `SessionRegistry` to use.
    fn get_session_registry(&mut self, http: &mut H) -> Arc<dyn SessionRegistry> {
        if let Some(ref registry) = self.session_registry {
            return registry.clone();
        }

        let registry = http.shared_object::<ApplicationContext>().and_then(|ctx| {
            ctx.get_single_option::<Arc<dyn SessionRegistry>>()
                .map(Clone::clone)
        });
        if let Some(registry) = registry {
            self.session_registry = Some(registry.clone());
            return registry;
        }

        let session_registry = SessionRegistryImpl::default();
        self.register_delegate_application_listener(http, Arc::new(session_registry.clone()));
        let registry = Arc::new(session_registry);
        self.session_registry = Some(registry.clone());
        registry
    }

    /// Registers a delegate application listener.
    fn register_delegate_application_listener(
        &self,
        http: &mut H,
        delegate: Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>,
    ) {
        let delegating = http
            .shared_object_mut::<ApplicationContext>()
            .and_then(|ctx| ctx.get_single_option_mut::<DelegatingApplicationListener>());
        if let Some(delegating) = delegating {
            let smart_listener = GenericApplicationListenerAdapter::new(delegate);
            delegating.add_listener(Arc::new(smart_listener));
        }
    }

    /// Returns true if the number of concurrent sessions per user should be restricted.
    fn is_concurrent_session_control_enabled(&self) -> bool {
        self.session_limit.is_some()
    }

    /// Creates the default `SessionAuthenticationStrategy` for session fixation.
    fn create_default_session_fixation_protection_strategy(
    ) -> Arc<dyn SessionAuthenticationStrategy> {
        Arc::new(ChangeSessionIdAuthenticationStrategy::default())
    }
}

impl<H> Deref for SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }
    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn init(&mut self, http: &mut H) {
        let security_context_repository =
            http.shared_object::<Arc<dyn SecurityContextRepository>>();
        let stateless = self.is_stateless(http);

        if security_context_repository.is_none() {
            if stateless {
                http.set_shared_object::<Arc<dyn SecurityContextRepository>>(Arc::new(
                    RequestAttributeSecurityContextRepository::default(),
                ));
                self.session_management_security_context_repository =
                    Some(Arc::new(NullSecurityContextRepository::default()));
            } else {
                let mut http_security_repository = HttpSessionSecurityContextRepository::default();
                http_security_repository
                    .set_disable_url_rewriting(!self.enable_session_url_rewriting);
                http_security_repository
                    .set_allow_session_creation(self.is_allow_session_creation(http));
                let trust_resolver = http.shared_object::<Arc<dyn AuthenticationTrustResolver>>();
                if let Some(trust_resolver) = trust_resolver {
                    http_security_repository.set_trust_resolver(trust_resolver.to_owned());
                }
                let http_security_repository = Arc::new(http_security_repository);
                self.session_management_security_context_repository =
                    Some(http_security_repository.clone());
                let default_repository = Arc::new(DelegatingSecurityContextRepository::new(vec![
                    http_security_repository,
                    Arc::new(RequestAttributeSecurityContextRepository::default()),
                ]));
                http.set_shared_object::<Arc<dyn SecurityContextRepository>>(default_repository);
            }
        } else {
            self.session_management_security_context_repository =
                security_context_repository.map(Clone::clone);
        }

        let request_cache = http.shared_object::<Arc<dyn RequestCache>>();
        if request_cache.is_none() && stateless {
            http.set_shared_object::<Arc<dyn RequestCache>>(Arc::new(NullRequestCache::default()));
        }

        let session_authentication_strategy = self.get_session_authentication_strategy(http);
        http.set_shared_object::<Arc<dyn SessionAuthenticationStrategy>>(
            session_authentication_strategy,
        );

        if let Some(invalid_session_strategy) = self.get_invalid_session_strategy() {
            http.set_shared_object::<Arc<dyn InvalidSessionStrategy>>(invalid_session_strategy);
        }
    }

    fn configure(&mut self, http: &mut H) {
        if let Some(filter) = self.create_session_management_filter(http) {
            http.add_filter(filter);
        }
        if self.is_concurrent_session_control_enabled() {
            let concurrent_session_filter = self.create_concurrency_filter(http);
            // let concurrent_session_filter = self.base.post_process(concurrent_session_filter);
            http.add_filter(concurrent_session_filter);
        }
        if self.session_policy == Some(SessionCreationPolicy::Always) {
            http.add_filter(ForceEagerSessionCreationFilter::default());
        }
    }
}

impl<H> Default for SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        let default_strategy = Self::create_default_session_fixation_protection_strategy();
        Self {
            default_session_fixation_strategy: default_strategy.clone(),
            session_fixation_authentication_strategy: Some(default_strategy),
            session_authentication_strategy: None,
            provided_session_authentication_strategy: None,
            invalid_session_strategy: None,
            expired_session_strategy: None,
            session_authentication_strategies: Vec::new(),
            session_registry: None,
            session_limit: None,
            expired_url: None,
            max_sessions_prevents_login: false,
            session_policy: None,
            enable_session_url_rewriting: false,
            invalid_session_url: None,
            session_authentication_error_url: None,
            session_authentication_failure_handler: None,
            properties_that_require_implicit_authentication: HashSet::new(),
            require_explicit_authentication_strategy: None,
            session_management_security_context_repository: Some(Arc::new(
                HttpSessionSecurityContextRepository::default(),
            )),

            base: Default::default(),
        }
    }
}

/// Allows configuring SessionFixation protection.
pub struct SessionFixationConfigurer<'a, H>
where
    H: HttpSecurityBuilder<H>,
{
    parent: &'a mut SessionManagementConfigurer<H>,
}

impl<'a, H> SessionFixationConfigurer<'a, H>
where
    H: HttpSecurityBuilder<H>,
{
    fn new(parent: &'a mut SessionManagementConfigurer<H>) -> Self {
        Self { parent }
    }

    /// Specifies that a new session should be created, but the session attributes from
    /// the original `HttpSession` should not be retained.
    pub fn new_session(&mut self) -> &mut SessionManagementConfigurer<H> {
        let mut session_fixation_protection_strategy = SessionFixationProtectionStrategy::default();
        session_fixation_protection_strategy.set_migrate_session_attributes(false);
        self.parent
            .set_session_fixation_authentication_strategy(Arc::new(
                session_fixation_protection_strategy,
            ));
        self.parent
    }

    /// Specifies that a new session should be created and the session attributes from
    /// the original `HttpSession` should be retained.
    pub fn migrate_session(&mut self) -> &mut SessionManagementConfigurer<H> {
        self.parent
            .set_session_fixation_authentication_strategy(Arc::new(
                SessionFixationProtectionStrategy::default(),
            ));
        self.parent
    }

    /// Specifies that the Servlet container-provided session fixation protection should
    /// be used. When a session authenticates, the Servlet method
    /// `HttpServletRequest#changeSessionId()` is called to change the session ID and
    /// retain all session attributes.
    pub fn change_session_id(&mut self) -> &mut SessionManagementConfigurer<H> {
        self.parent
            .set_session_fixation_authentication_strategy(Arc::new(
                ChangeSessionIdAuthenticationStrategy::default(),
            ));
        self.parent
    }

    /// Specifies that no session fixation protection should be enabled. This may be
    /// useful when utilizing other mechanisms for protecting against session fixation.
    /// For example, if application container session fixation protection is already in
    /// use. Otherwise, this option is not recommended.
    pub fn none(&mut self) -> &mut SessionManagementConfigurer<H> {
        self.parent.session_fixation_authentication_strategy = None;
        self.parent
    }
}

/// Allows configuring controlling of multiple sessions.
pub struct ConcurrencyControlConfigurer<'a, H>
where
    H: HttpSecurityBuilder<H>,
{
    parent: &'a mut SessionManagementConfigurer<H>,
}

impl<'a, H> ConcurrencyControlConfigurer<'a, H>
where
    H: HttpSecurityBuilder<H>,
{
    fn new(parent: &'a mut SessionManagementConfigurer<H>) -> Self {
        Self { parent }
    }

    /// Controls the maximum number of sessions for a user. The default is to allow any
    /// number of users.
    ///
    /// # Arguments
    ///
    /// * `maximum_sessions` - The maximum number of sessions for a user.
    pub fn maximum_sessions(self, maximum_sessions: i32) -> Self {
        self.parent.session_limit = Some(session_limit_of(maximum_sessions));
        self
    }

    /// Determines the behaviour when a session limit is detected.
    ///
    /// # Arguments
    ///
    /// * `session_limit` - The `SessionLimit` to check the maximum number of sessions
    ///   for a user.
    pub fn maximum_sessions_with_limit(self, session_limit: SessionLimit) -> Self {
        self.parent.session_limit = Some(session_limit);
        self
    }

    /// The URL to redirect to if a user tries to access a resource and their session has
    /// been expired due to too many sessions for the current user. The default is to
    /// write a simple error message to the response.
    ///
    /// # Arguments
    ///
    /// * `expired_url` - The URL to redirect to.
    pub fn expired_url(self, expired_url: impl Into<String>) -> Self {
        self.parent.expired_url = Some(expired_url.into());
        self
    }

    /// Determines the behaviour when an expired session is detected.
    ///
    /// # Arguments
    ///
    /// * `expired_session_strategy` - The `SessionInformationExpiredStrategy` to use
    ///   when an expired session is detected.
    pub fn expired_session_strategy(
        self,
        expired_session_strategy: Arc<dyn SessionInformationExpiredStrategy>,
    ) -> Self {
        self.parent.expired_session_strategy = Some(expired_session_strategy);
        self
    }

    /// If true, prevents a user from authenticating when the maximum number of sessions
    /// has been reached. Otherwise (default), the user who authenticates is allowed
    /// access and an existing user's session is expired. The user whose session is
    /// forcibly expired is sent to `expired_url`. The advantage of this approach is if a
    /// user accidentally does not log out, there is no need for an administrator to
    /// intervene or wait till their session expires.
    ///
    /// # Arguments
    ///
    /// * `max_sessions_prevents_login` - true to have an error at time of authentication,
    ///   else false (default).
    pub fn max_sessions_prevents_login(self, max_sessions_prevents_login: bool) -> Self {
        self.parent.max_sessions_prevents_login = max_sessions_prevents_login;
        self
    }

    /// Controls the `SessionRegistry` implementation used. The default is
    /// `SessionRegistryImpl` which is an in memory implementation.
    ///
    /// # Arguments
    ///
    /// * `session_registry` - The `SessionRegistry` to use.
    pub fn session_registry(self, session_registry: Arc<dyn SessionRegistry>) -> Self {
        self.parent.session_registry = Some(session_registry);
        self
    }

    /// Returns the parent `SessionManagementConfigurer` for further customization.
    pub fn and(self) -> &'a mut SessionManagementConfigurer<H> {
        self.parent
    }
}
