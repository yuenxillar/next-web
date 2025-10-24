use crate::core::{
    authz::authorization_error::AuthorizationError, cache::{cache_manager_aware::CacheManagerAware, default_cache_manager::DefaultCacheManager}, event::{event_bus_aware::EventBusAware, support::default_event_bus::DefaultEventBus}, session::{
        mgt::{session_context::SessionContext, session_manager::SessionManager}, Session, SessionError, SessionId
    }
};

#[derive(Clone)]
pub struct DefaultSessionManager {}

impl SessionManager for DefaultSessionManager {
    fn start(&self, context: &dyn SessionContext) -> Result<Box<dyn Session>, AuthorizationError> {
        todo!()
    }

    fn get_session(&self, id: SessionId) -> Result<std::sync::Arc<dyn Session>, SessionError> {
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
        Self {}
    }
}
