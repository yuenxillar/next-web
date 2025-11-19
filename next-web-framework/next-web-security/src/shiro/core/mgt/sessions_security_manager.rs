use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        required::Required,
    },
};

use crate::core::{
    authc::{
        authenticator::Authenticator, pam::modular_realm_authenticator::ModularRealmAuthenticator,
    },
    authz::{
        authorization_error::AuthorizationError, authorizer::Authorizer,
        modular_realm_authorizer::ModularRealmAuthorizer,
    },
    cache::{
        cache_manager::CacheManager, cache_manager_aware::CacheManagerAware,
        default_cache_manager::DefaultCacheManager,
    },
    event::{
        event_bus::EventBus, event_bus_aware::EventBusAware,
        support::default_event_bus::DefaultEventBus,
    },
    mgt::authorizing_security_manager::AuthorizingSecurityManager,
    realm::{simple_account_realm::SimpleAccountRealm, Realm},
    session::{
        mgt::{
            default_session_manager::DefaultSessionManager, session_context::SessionContext,
            session_manager::SessionManager,
        },
        Session, SessionError, SessionId,
    },
    util::destroyable::Destroyable,
};

#[derive(Clone)]
pub struct SessionsSecurityManager<
    S = DefaultSessionManager,
    A = ModularRealmAuthorizer,
    T = ModularRealmAuthenticator,
    R = SimpleAccountRealm,
    C = DefaultCacheManager,
    B = DefaultEventBus,
> {
    session_manager: S,
    authorizing_security_manager: AuthorizingSecurityManager<A, T, R, C, B>,
}

impl<S, A, T, R, C, B> SessionsSecurityManager<S, A, T, R, C, B>
where
    S: SessionManager,
{
    pub fn get_session_manager(&self) -> &S {
        &self.session_manager
    }
}

impl<S, A, T, R, C, B> SessionsSecurityManager<S, A, T, R, C, B>
where
    S: SessionManager + CacheManagerAware<C> + EventBusAware<B>,
    S: Clone,
    A: Authorizer,
    T: Authenticator,
    R: Realm + CacheManagerAware<C> + EventBusAware<B>,
    R: Clone,
    C: CacheManager + EventBusAware<B> + Clone,
    B: EventBus + Clone,
{
    pub fn after_session_manager_set(&mut self) {
        self.apply_cache_manager_to_session_manager();
        self.apply_event_bus_to_session_manager();
    }

    pub fn after_cache_manager_set(&mut self) {
        self.authorizing_security_manager
            .get_mut_object()
            .get_mut_object()
            .after_cache_manager_set();

        self.apply_cache_manager_to_session_manager();
    }

    pub fn set_session_manager(&mut self, session_manager: S) {
        self.session_manager = session_manager;
        self.after_session_manager_set();
    }

    pub fn after_event_bus_set(&mut self) {
        self.authorizing_security_manager
            .get_mut_object()
            .get_mut_object()
            .after_event_bus_set();

        self.apply_event_bus_to_session_manager();
    }

    pub fn apply_cache_manager_to_session_manager(&mut self) {
        let cache_manager = self
            .authorizing_security_manager
            .get_object()
            .get_object()
            .get_object()
            .get_cache_manager();
        if let Some(cache_manager) = cache_manager {
            self.session_manager
                .set_cache_manager(cache_manager.to_owned());
        }
    }

    pub fn apply_event_bus_to_session_manager(&mut self) {
        let event_bus = self
            .authorizing_security_manager
            .get_object()
            .get_object()
            .get_object()
            .get_event_bus();

        if let Some(event_bus) = event_bus {
            self.session_manager.set_event_bus(event_bus.to_owned());
        }
    }
}

#[async_trait]
impl<S, A, T, R, C, B> SessionManager for SessionsSecurityManager<S, A, T, R, C, B>
where
    S: SessionManager,
    A: Send + Sync,
    T: Send + Sync,
    R: Send + Sync,
    C: Send + Sync,
    B: Send + Sync,
{
    async fn start(
        &self,
        context: &dyn SessionContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<Box<dyn Session>, AuthorizationError> {
        self.session_manager.start(context, req, resp).await
    }

    async fn get_session(&self, id: &SessionId) -> Result<Arc<dyn Session>, SessionError> {
        self.session_manager.get_session(id).await
    }
}

impl<S, A, T, R, C, B> Required<AuthorizingSecurityManager<A, T, R, C, B>>
    for SessionsSecurityManager<S, A, T, R, C, B>
where
    A: Authorizer,
    T: Authenticator,
    R: Realm + CacheManagerAware<C> + EventBusAware<B>,
    R: Clone,
    C: CacheManager + Clone,
    B: EventBus + Clone,
{
    fn get_object(&self) -> &AuthorizingSecurityManager<A, T, R, C, B> {
        &self.authorizing_security_manager
    }

    fn get_mut_object(&mut self) -> &mut AuthorizingSecurityManager<A, T, R, C, B> {
        &mut self.authorizing_security_manager
    }
}

impl<S, A, T, R, C, B> Destroyable for SessionsSecurityManager<S, A, T, R, C, B>
where
    S: Destroyable,
    A: Authorizer + Destroyable,
    T: Authenticator + Destroyable,
    R: Realm,
    C: CacheManager,
    B: Destroyable + Default + EventBus,
{
    fn destroy(self) {
        self.session_manager.destroy();
        self.authorizing_security_manager.destroy();
    }
}

impl<S, A, T, R, C, B> Default for SessionsSecurityManager<S, A, T, R, C, B>
where
    S: SessionManager + CacheManagerAware<C> + EventBusAware<B>,
    S: Default + Clone,
    A: Default + Authorizer,
    T: Default + Authenticator,
    R: Default + Clone,
    R: Realm + CacheManagerAware<C> + EventBusAware<B>,
    C: Default + Clone,
    C: CacheManager + EventBusAware<B>,
    B: Default + Clone + EventBus,
{
    fn default() -> Self {
        let mut manager = Self {
            session_manager: Default::default(),
            authorizing_security_manager: Default::default(),
        };

        manager.apply_cache_manager_to_session_manager();
        manager
    }
}
