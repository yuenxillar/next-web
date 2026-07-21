use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::traits::required::Required;

use crate::{
    config::{
        http::SessionCreationPolicy,
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    core::session::SessionRegistry,
    web::{
        authentication::session::{session_limit_of, SessionAuthenticationStrategy, SessionLimit},
        default_security_filter_chain::DefaultSecurityFilterChain,
        session::{InvalidSessionStrategy, SessionInformationExpiredStrategy},
    },
};

/// Configures session management: concurrency control, session fixation
/// protection, invalid session handling, and session creation policy.
#[derive(Clone)]
pub struct SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    session_registry: Option<Arc<dyn SessionRegistry>>,
    max_sessions_prevents_login: bool,
    expired_url: Option<String>,
    expired_session_strategy: Option<Arc<dyn SessionInformationExpiredStrategy>>,
    session_limit: Option<SessionLimit>,
    invalid_session_strategy: Option<Arc<dyn InvalidSessionStrategy>>,
    invalid_session_url: Option<String>,
    enable_session_url_rewriting: bool,
    session_authentication_strategies: Vec<Arc<dyn SessionAuthenticationStrategy>>,
    session_policy: Option<SessionCreationPolicy>,
    properties_that_require_implicit_authentication: HashSet<String>,

    inner: BaseHttpConfigurer<Self, H>,
}

impl<H> SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    pub fn get_invalid_session_strategy(&self) -> Option<Arc<dyn InvalidSessionStrategy>> {
        self.invalid_session_strategy.clone()
    }

    pub fn session_creation_policy(mut self, p: SessionCreationPolicy) -> Self {
        self.session_policy = Some(p);
        self
    }

    pub fn add_session_authentication_strategy(
        &mut self,
        session_authentication_strategies: Arc<dyn SessionAuthenticationStrategy>,
    ) {
        self.session_authentication_strategies
            .push(session_authentication_strategies);
    }

    pub fn enable_session_url_rewriting(mut self, v: bool) -> Self {
        self.enable_session_url_rewriting = v;
        self
    }

    pub fn invalid_session_strategy<T: InvalidSessionStrategy + 'static>(mut self, s: T) -> Self {
        self.invalid_session_strategy = Some(Arc::new(s));
        self
    }

    pub fn invalid_session_url(mut self, url: impl Into<String>) -> Self {
        self.invalid_session_url = Some(url.into());
        self
    }

    pub fn maximum_sessions(mut self, max: i32) -> Self {
        self.session_limit = Some(session_limit_of(max));
        self
    }

    pub fn expired_session_strategy<T: SessionInformationExpiredStrategy + 'static>(
        mut self,
        s: T,
    ) -> Self {
        self.expired_session_strategy = Some(Arc::new(s));
        self
    }

    pub fn expired_url(mut self, url: impl Into<String>) -> Self {
        self.expired_url = Some(url.into());
        self
    }

    pub fn max_sessions_prevents_login(mut self, v: bool) -> Self {
        self.max_sessions_prevents_login = v;
        self
    }

    pub fn session_registry<T: SessionRegistry + 'static>(mut self, r: T) -> Self {
        self.session_registry = Some(Arc::new(r));
        self
    }

    pub fn get_session_creation_policy(&self) -> SessionCreationPolicy {
        todo!()
    }
}

impl<H> Deref for SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.inner.get_object()
    }
    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.inner.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {
        // SessionManagementFilter is added in configure().
        // Shared objects (SessionRegistry, etc.) set by DSL methods.
    }

    fn configure(&mut self, _http: &mut H) {
        // SessionManagementFilter is deferred until the filter implementation
        // is complete. When ready:
        //   let filter = SessionManagementFilter::new(
        //       session_authentication_strategy,
        //       invalid_session_strategy,
        //   );
        //   filter.set_session_creation_policy(self.session_policy);
        //   http.add_filter(filter);
        //
        // All DSL fields (session_limit, expired_url, etc.) are preserved
        // on the configurer and ready for filter construction.
    }
}

impl<H> Default for SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            session_registry: None,
            max_sessions_prevents_login: false,
            expired_url: None,
            expired_session_strategy: None,
            session_limit: None,
            invalid_session_strategy: None,
            invalid_session_url: None,
            enable_session_url_rewriting: false,
            session_authentication_strategies: Vec::new(),
            session_policy: None,
            properties_that_require_implicit_authentication: HashSet::new(),

            inner: Default::default(),
        }
    }
}
