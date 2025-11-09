use next_web_core::async_trait;

use crate::core::{
    authz::authorization_error::AuthorizationError,
    cache::{
        cache_manager::CacheManager, cache_manager_aware::CacheManagerAware,
        default_cache_manager::DefaultCacheManager,
    },
    event::{event_bus_aware::EventBusAware, support::default_event_bus::DefaultEventBus},
    session::{
        expired_session_error::ExpiredSessionError,
        mgt::{
            default_validating_session_manager::DefaultValidatingSessionManager,
            eis::{memory_session_dao::MemorySessionDAO, session_dao::SessionDAO},
            native_session_manager::NativeSessionManagerExt,
            session_context::SessionContext,
            session_factory::SessionFactory,
            session_manager::SessionManager,
            simple_session::SimpleSession,
            simple_session_factory::SimpleSessionFactory,
            validating_session_manager::ValidatingSessionManagerExt,
        },
        Session, SessionError, SessionId,
    },
};
use std::{
    any::Any,
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Clone)]
pub struct DefaultSessionManager {
    session_dao: Arc<dyn SessionDAO>,
    session_factory: Arc<dyn SessionFactory>,
    cache_manager: Option<Arc<dyn CacheManager>>,
    delete_invalid_sessions: bool,

    pub(crate) validating_session_manager: DefaultValidatingSessionManager,
}

impl DefaultSessionManager {
    pub fn set_session_dao<T>(&mut self, session_dao: T)
    where
        T: SessionDAO + 'static,
    {
        self.session_dao = Arc::new(session_dao);
    }

    pub fn get_session_dao(&self) -> &Arc<dyn SessionDAO> {
        &self.session_dao
    }

    pub fn get_session_factory(&self) -> &Arc<dyn SessionFactory> {
        &self.session_factory
    }

    pub fn set_session_factory<T>(&mut self, session_factory: T)
    where
        T: SessionFactory + 'static,
    {
        self.session_factory = Arc::new(session_factory);
    }

    pub fn is_delete_invalid_sessions(&self) -> bool {
        self.delete_invalid_sessions
    }

    pub fn set_delete_invalid_sessions(&mut self, delete_invalid_sessions: bool) {
        self.delete_invalid_sessions = delete_invalid_sessions;
    }

    pub async fn retrieve_session_from_data_source(
        &self,
        session_id: &SessionId,
    ) -> Option<&Arc<dyn Session>> {
        self.session_dao.read(session_id).await
    }

    pub async fn get_active_sessions(&self) -> Vec<&Arc<dyn Session>> {
        self.session_dao.get_active_sessions().await
    }
}

impl DefaultSessionManager {
    pub async fn create(&mut self, session: Arc<dyn Session>) {
        self.session_dao.create(session).await;
    }

    pub fn new_session_instance(&self, ctx: &dyn SessionContext) -> Arc<dyn Session> {
        self.session_factory.create_session(ctx)
    }
}

impl SessionManager for DefaultSessionManager {
    fn start(&self, context: &dyn SessionContext) -> Result<Box<dyn Session>, AuthorizationError> {
        todo!()
    }

    fn get_session(&self, id: &SessionId) -> Result<std::sync::Arc<dyn Session>, SessionError> {
        todo!()
    }
}

#[async_trait]
impl NativeSessionManagerExt for DefaultSessionManager {
    async fn on_stop(&self, session: &Arc<dyn Session>) {
        if let Some(simple_session) = (session as &dyn Any).downcast_ref::<SimpleSession>() {
            let stop_tt = simple_session.stop_time_stamp();
            simple_session.set_last_access_time(stop_tt);
        }

        self.on_change(session).await;
    }

    async fn after_stopped(&self, session: &mut dyn Session) {
        if self.is_delete_invalid_sessions() {
            self.session_dao.delete(session).await;
        }
    }

    async fn on_change(&self, session: &Arc<dyn Session>) {
        self.session_dao.update(session.clone()).await;
    }

    fn do_get_session(
        &self,
        session_id: &SessionId,
        ctx: &dyn SessionContext,
    ) -> Result<Arc<dyn Session>, SessionError> {
        todo!()
    }
}

#[async_trait]
impl ValidatingSessionManagerExt for DefaultSessionManager {
    async fn do_create_session(
        &self,
        ctx: &dyn SessionContext,
    ) -> Result<Arc<dyn Session>, AuthorizationError> {
        let session = self.new_session_instance(ctx);
        self.session_dao.create(session.clone()).await;

        Ok(session)
    }

    async fn on_expiration(&self, session: &Arc<dyn Session>, _error: ExpiredSessionError) {
        if let Some(simple_session) = (session as &dyn Any).downcast_ref::<SimpleSession>() {
            simple_session.set_expired(true);
        }

        self.on_change(session).await;
    }

    async fn after_expired(&self, session: &dyn Session) {
        if self.is_delete_invalid_sessions() {
            self.session_dao.delete(session).await;
        }
    }
}

impl CacheManagerAware<DefaultCacheManager> for DefaultSessionManager {
    fn set_cache_manager(&mut self, cache_manager: DefaultCacheManager) {
        self.cache_manager = Some(Arc::new(cache_manager));
    }
}

impl EventBusAware<DefaultEventBus> for DefaultSessionManager {
    fn set_event_bus(&mut self, event_bus: DefaultEventBus) {
        self.validating_session_manager
            .native_session_manager
            .set_event_bus(event_bus);
    }
}

impl Default for DefaultSessionManager {
    fn default() -> Self {
        Self {
            session_dao: Arc::new(MemorySessionDAO::default()),
            session_factory: Arc::new(SimpleSessionFactory::default()),
            cache_manager: None,
            delete_invalid_sessions: true,

            validating_session_manager: Default::default(),
        }
    }
}

impl Deref for DefaultSessionManager {
    type Target = DefaultValidatingSessionManager;

    fn deref(&self) -> &Self::Target {
        &self.validating_session_manager
    }
}

impl DerefMut for DefaultSessionManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.validating_session_manager
    }
}
