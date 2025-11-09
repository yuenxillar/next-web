use crate::core::event::event_bus::EventBus;
use crate::core::event::event_bus_aware::EventBusAware;
use crate::core::event::support::default_event_bus::DefaultEventBus;
use crate::core::session::mgt::delegating_session::DelegatingSession;
use crate::core::session::mgt::immutable_proxied_session::ImmutableProxiedSession;
use crate::core::session::mgt::native_session_manager::NativeSessionManager;
use crate::core::session::session_listener::SessionListener;
use crate::core::{
    authz::authorization_error::AuthorizationError,
    session::{
        mgt::{session_context::SessionContext, session_manager::SessionManager},
        Session, SessionError, SessionId,
    },
    util::object::Object,
};
use std::{collections::HashSet, sync::Arc};

#[derive(Clone)]
pub struct DefaultNativeSessionManager {
    event_bus: Option<Arc<dyn EventBus>>,
    listeners: Vec<Arc<dyn SessionListener>>,
}

impl DefaultNativeSessionManager {
    pub fn set_session_listeners(&mut self, listeners: Vec<Arc<dyn SessionListener>>) {
        self.listeners = listeners;
    }

    pub fn get_session_listeners(&self) -> &Vec<Arc<dyn SessionListener>> {
        &self.listeners
    }

    pub fn get_event_bus(&self) -> Option<&Arc<dyn EventBus>> {
        self.event_bus.as_ref()
    }

    pub fn publish_event(&self, event: Object) {
        if let Some(event_bus) = self.event_bus.as_ref() {
            event_bus.publish(event);
        }
    }

    pub fn notify_start(&self, session: &Box<dyn Session>) {
        for listener in self.listeners.iter() {
            listener.on_start(session.as_ref());
        }
    }

    pub fn notify_stop(&self, session: &Box<dyn Session>) {
        let for_notification = self.before_invalid_notification(session.to_owned());
        for listener in self.listeners.iter() {
            listener.on_stop(&for_notification);
        }
    }

    pub fn notify_expiration(&self, session: &Box<dyn Session>) {
        let for_notification = self.before_invalid_notification(session.to_owned());
        for listener in self.listeners.iter() {
            listener.on_expiration(&for_notification);
        }
    }

    pub fn before_invalid_notification(
        &self,
        session: Box<dyn Session>,
    ) -> ImmutableProxiedSession {
        ImmutableProxiedSession::new(session)
    }

    fn lookup_session(&self, id: &SessionId) -> Result<Arc<dyn Session>, SessionError> {
        todo!()
    }

    pub fn apply_global_session_timeout(&self, session: &mut dyn Session) {
        session.set_timeout(max_idle_time_in_millis);
        self.on_change(session);
    }

    pub fn create_exposed_session(&self, session: &dyn Session) -> DelegatingSession {
        DelegatingSession::new(session.id().clone())
    }
}

impl NativeSessionManager for DefaultNativeSessionManager {
    fn start_time_stamp(&self, session_id: &SessionId) -> i64 {
        todo!()
    }

    fn last_access_time(&self, session_id: &SessionId) -> i64 {
        todo!()
    }

    fn is_valid(&self, session_id: &SessionId) -> bool {
        todo!()
    }

    fn check_valid(&self, session_id: &SessionId) -> Result<(), SessionError> {
        todo!()
    }

    fn timeout(&self, session_id: &SessionId) -> Result<i64, SessionError> {
        todo!()
    }

    fn set_timeout(
        &mut self,
        session_id: &SessionId,
        max_idle_time_in_millis: i64,
    ) -> Result<(), SessionError> {
        todo!()
    }

    fn touch(&self, session_id: &SessionId) -> Result<(), SessionError> {
        todo!()
    }

    fn host(&self, session_id: &SessionId) -> Option<&str> {
        todo!()
    }

    fn stop(&self, session_id: &SessionId) -> Result<(), SessionError> {
        todo!()
    }

    fn attribute_keys(&self, session_id: &SessionId) -> Result<HashSet<String>, SessionError> {
        todo!()
    }

    fn attribute(&self, session_id: &SessionId, key: &str) -> Result<Object, SessionError> {
        todo!()
    }

    fn set_attribute(
        &self,
        session_id: &SessionId,
        key: &str,
        value: Object,
    ) -> Result<(), SessionError> {
        todo!()
    }

    fn remove_attribute(&self, session_id: &SessionId, key: &str) -> Result<Object, SessionError> {
        todo!()
    }
}

impl EventBusAware<DefaultEventBus> for DefaultNativeSessionManager {
    fn set_event_bus(&mut self, event_bus: DefaultEventBus) {
        self.event_bus = Some(Arc::new(event_bus));
    }
}

impl Default for DefaultNativeSessionManager {
    fn default() -> Self {
        Self {
            event_bus: None,
            listeners: Default::default(),
        }
    }
}
