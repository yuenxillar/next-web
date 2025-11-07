use std::sync::Arc;

use crate::core::{
    authz::authorization_error::AuthorizationError,
    cache::{
        cache_manager::CacheManager, cache_manager_aware::CacheManagerAware,
        default_cache_manager::DefaultCacheManager,
    },
    event::{event_bus_aware::EventBusAware, support::default_event_bus::DefaultEventBus},
    session::{
        mgt::{
            eis::{memory_session_dao::MemorySessionDAO, session_dao::SessionDAO},
            session_context::SessionContext,
            session_factory::SessionFactory,
            session_manager::SessionManager,
            simple_session_factory::SimpleSessionFactory,
        },
        Session, SessionError, SessionId,
    },
};

#[derive(Clone)]
pub struct DefaultSessionManager {
    session_dao: Arc<dyn SessionDAO>,
    session_factory: Arc<dyn SessionFactory>,
    cache_manager: Option<Arc<dyn CacheManager>>,
    delete_invalid_sessions: bool,
}

impl SessionManager for DefaultSessionManager {
    fn start(&self, context: &dyn SessionContext) -> Result<Box<dyn Session>, AuthorizationError> {
        todo!()
    }

    fn get_session(&self, id: &SessionId) -> Result<std::sync::Arc<dyn Session>, SessionError> {
        todo!()
    }
}

impl CacheManagerAware<DefaultCacheManager> for DefaultSessionManager {
    fn set_cache_manager(&mut self, cache_manager: DefaultCacheManager) {
        todo!()
    }
}

impl EventBusAware<DefaultEventBus> for DefaultSessionManager {
    fn set_event_bus(&mut self, event_bus: DefaultEventBus) {
        todo!()
    }
}

impl Default for DefaultSessionManager {
    fn default() -> Self {
        Self {
            session_dao: Arc::new(MemorySessionDAO::default()),
            session_factory: Arc::new(SimpleSessionFactory::default()),
            cache_manager: None,
            delete_invalid_sessions: true,
        }
    }
}
