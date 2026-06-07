use std::{collections::HashSet, sync::Arc};

use next_web_core::traits::required::Required;

use crate::{
    config::{
        http::SessionCreationPolicy,
        security_configurer::SecurityConfigurer,
        web::{
            configurers::base_http_configurer::BaseHttpConfigurer,
            http_security_builder::HttpSecurityBuilder,
        },
    },
    core::session::SessionRegistry,
    web::{
        authentication::session::{session_limit_of, SessionAuthenticationStrategy, SessionLimit},
        default_security_filter_chain::DefaultSecurityFilterChain,
        session::{InvalidSessionStrategy, SessionInformationExpiredStrategy},
    },
};

#[derive(Default)]
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

    base_http_configurer: BaseHttpConfigurer<Self, H>,
}

impl<H> SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    pub fn get_invalid_session_strategy(&self) -> Option<Arc<dyn InvalidSessionStrategy>> {
        self.invalid_session_strategy.clone()
    }

    pub fn session_creation_policy(
        mut self,
        session_creation_policy: SessionCreationPolicy,
    ) -> Self {
        self.properties_that_require_implicit_authentication
            .insert(format!(
                "sessionCreationPolicy = {:?}",
                session_creation_policy
            ));
        self.session_policy = Some(session_creation_policy);

        self
    }

    pub fn add_session_authentication_strategy(
        mut self,
        session_authentication_strategy: Arc<dyn SessionAuthenticationStrategy>,
    ) -> Self {
        self.session_authentication_strategies
            .push(session_authentication_strategy);

        self
    }

    pub fn enable_session_url_rewriting(mut self, enable_session_url_rewriting: bool) -> Self {
        self.enable_session_url_rewriting = enable_session_url_rewriting;

        self
    }

    pub fn invalid_session_strategy<T>(mut self, invalid_session_strategy: T) -> Self
    where
        T: InvalidSessionStrategy,
        T: 'static,
    {
        self.properties_that_require_implicit_authentication
            .insert(format!(
                "invalidSessionStrategy = {}",
                std::any::type_name::<T>()
            ));
        self.invalid_session_strategy = Some(Arc::new(invalid_session_strategy));

        self
    }

    pub fn invalid_session_url(mut self, invalid_session_url: impl Into<String>) -> Self {
        let invalid_session_url = invalid_session_url.into();
        self.properties_that_require_implicit_authentication
            .insert(format!("invalidSessionUrl = {}", &invalid_session_url));
        self.invalid_session_url = Some(invalid_session_url);

        self
    }

    pub fn maximum_sessions(mut self, maximum_sessions: i32) -> Self {
        self.properties_that_require_implicit_authentication
            .insert(format!("maximumSessions = {}", maximum_sessions));
        self.session_limit = Some(session_limit_of(maximum_sessions));

        self
    }

    pub fn expired_session_strategy<T>(mut self, expired_session_strategy: T) -> Self
    where
        T: SessionInformationExpiredStrategy,
        T: 'static,
    {
        self.expired_session_strategy = Some(Arc::new(expired_session_strategy));

        self
    }

    pub fn expired_url(mut self, expired_url: impl Into<String>) -> Self {
        self.expired_url = Some(expired_url.into());

        self
    }

    pub fn max_sessions_prevents_login(mut self, max_sessions_prevents_login: bool) -> Self {
        self.max_sessions_prevents_login = max_sessions_prevents_login;

        self
    }

    pub fn session_registry<T>(mut self, session_registry: T) -> Self
    where
        T: SessionRegistry,
        T: 'static,
    {
        self.session_registry = Some(Arc::new(session_registry));

        self
    }
}

impl<H> Required<BaseHttpConfigurer<SessionManagementConfigurer<H>, H>>
    for SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<SessionManagementConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<SessionManagementConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for SessionManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, builer: &mut H) {
        todo!()
    }

    fn configure(&mut self, builer: &mut H) {
        todo!()
    }
}
