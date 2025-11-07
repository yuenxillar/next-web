use std::sync::Arc;

use crate::{
    core::{
        cache::{
            cache_manager_aware::CacheManagerAware, default_cache_manager::DefaultCacheManager,
        },
        event::{event_bus_aware::EventBusAware, support::default_event_bus::DefaultEventBus},
        session::mgt::{
            default_session_manager::DefaultSessionManager, session_manager::SessionManager,
        },
    },
    web::{Cookie, SimpleCookie},
};

#[derive(Clone)]
pub struct DefaultWebSessionManager {
    session_id_cookie: Arc<dyn Cookie>,
    default_session_manager: DefaultSessionManager,
    session_id_cookie_enabled: bool,
    session_id_url_rewriting_enabled: bool,
}

impl SessionManager for DefaultWebSessionManager {
    fn start(
        &self,
        context: &dyn crate::core::session::mgt::session_context::SessionContext,
    ) -> Result<
        Box<dyn crate::core::session::Session>,
        crate::core::authz::authorization_error::AuthorizationError,
    > {
        todo!()
    }

    fn get_session(
        &self,
        id: &crate::core::session::SessionId,
    ) -> Result<std::sync::Arc<dyn crate::core::session::Session>, crate::core::session::SessionError>
    {
        todo!()
    }
}

impl EventBusAware<DefaultEventBus> for DefaultWebSessionManager {
    fn set_event_bus(&mut self, event_bus: DefaultEventBus) {
        todo!()
    }
}

impl CacheManagerAware<DefaultCacheManager> for DefaultWebSessionManager {
    fn set_cache_manager(&mut self, cache_manager: DefaultCacheManager) {
        self.default_session_manager
            .set_cache_manager(cache_manager);
    }
}

impl Default for DefaultWebSessionManager {
    fn default() -> Self {
        Self {
            session_id_cookie: Arc::new(SimpleCookie::default()),
            default_session_manager: Default::default(),
            session_id_cookie_enabled: true,
            session_id_url_rewriting_enabled: false,
        }
    }
}
