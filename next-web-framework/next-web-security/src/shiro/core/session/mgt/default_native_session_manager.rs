use crate::core::event::event_bus::EventBus;
use crate::core::event::event_bus_aware::EventBusAware;
use crate::core::event::support::default_event_bus::DefaultEventBus;
use crate::core::session::mgt::delegating_session::DelegatingSession;
use crate::core::session::mgt::immutable_proxied_session::ImmutableProxiedSession;
use crate::core::session::session_listener::SessionListener;
use crate::core::{session::Session, util::object::Object};
use std::sync::Arc;

#[derive(Clone)]
pub struct DefaultNativeSessionManager {
    event_bus: Option<Arc<dyn EventBus>>,
    listeners: Vec<Arc<dyn SessionListener>>,
}

impl DefaultNativeSessionManager {
    pub const DEFAULT_GLOBAL_SESSION_TIMEOUT: i32 = 1800000;

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

    pub fn notify_start(&self, session: &dyn Session) {
        for listener in self.listeners.iter() {
            listener.on_start(session);
        }
    }

    pub fn notify_stop(&self, session: Arc<dyn Session>) {
        let for_notification = self.before_invalid_notification(session);
        for listener in self.listeners.iter() {
            listener.on_stop(&for_notification);
        }
    }

    pub fn notify_expiration(&self, session: Arc<dyn Session>) {
        let for_notification = self.before_invalid_notification(session);
        for listener in self.listeners.iter() {
            listener.on_expiration(&for_notification);
        }
    }

    pub fn before_invalid_notification(
        &self,
        session: Arc<dyn Session>,
    ) -> ImmutableProxiedSession {
        ImmutableProxiedSession::new(session)
    }

    pub fn apply_global_session_timeout(&self, session: &dyn Session) {
        session.set_timeout(Self::DEFAULT_GLOBAL_SESSION_TIMEOUT as i64).ok();
    }

    pub fn create_exposed_session(&self, session: &dyn Session) -> DelegatingSession {
        DelegatingSession::new(session.id().clone())
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
