use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use chrono::Utc;

use super::{
    session_events::SessionEvent, session_information::SessionInformation,
    session_registry::SessionRegistry,
};

#[derive(Clone, Default)]
pub struct SessionRegistryImpl {
    principals: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    session_ids: Arc<RwLock<HashMap<String, SessionInformation>>>,
}

impl SessionRegistryImpl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_application_event(&self, event: SessionEvent) {
        match event {
            SessionEvent::Destroyed(event) => self.remove_session_information(event.id()),
            SessionEvent::IdChanged(event) => {
                if let Some(info) = self.session_information(event.old_session_id()) {
                    let principal = info.principal();
                    self.remove_session_information(event.old_session_id());
                    self.register_new_session(event.new_session_id(), principal);
                }
            }
            SessionEvent::Created(_) => {}
        }
    }
}

impl SessionRegistry for SessionRegistryImpl {
    fn all_principals(&self) -> Vec<String> {
        let mut principals = self
            .principals
            .read()
            .map(|principals| principals.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        principals.sort();
        principals
    }

    fn all_sessions(
        &self,
        principal: &str,
        include_expired_sessions: bool,
    ) -> Vec<SessionInformation> {
        let session_ids = self
            .principals
            .read()
            .ok()
            .and_then(|principals| principals.get(principal).cloned())
            .unwrap_or_default();

        session_ids
            .into_iter()
            .filter_map(|session_id| self.session_information(&session_id))
            .filter(|info| include_expired_sessions || !info.is_expired())
            .collect()
    }

    fn session_information(&self, session_id: &str) -> Option<SessionInformation> {
        assert!(
            !session_id.trim().is_empty(),
            "SessionId required as per interface contract"
        );
        self.session_ids.read().ok()?.get(session_id).cloned()
    }

    fn refresh_last_request(&self, session_id: &str) {
        assert!(
            !session_id.trim().is_empty(),
            "SessionId required as per interface contract"
        );
        if let Ok(mut sessions) = self.session_ids.write() {
            if let Some(info) = sessions.get_mut(session_id) {
                info.refresh_last_request();
            }
        }
    }

    fn register_new_session(&self, session_id: &str, principal: &str) {
        let session_id = session_id.to_string();
        let principal = principal.to_string();
        assert!(
            !session_id.trim().is_empty(),
            "SessionId required as per interface contract"
        );
        assert!(
            !principal.trim().is_empty(),
            "Principal required as per interface contract"
        );

        if self.session_information(&session_id).is_some() {
            self.remove_session_information(&session_id);
        }

        if let Ok(mut sessions) = self.session_ids.write() {
            sessions.insert(
                session_id.clone(),
                SessionInformation::new(principal.clone(), session_id.clone(), Utc::now()),
            );
        }
        if let Ok(mut principals) = self.principals.write() {
            principals.entry(principal).or_default().insert(session_id);
        }
    }

    fn remove_session_information(&self, session_id: &str) {
        assert!(
            !session_id.trim().is_empty(),
            "SessionId required as per interface contract"
        );
        let Some(info) = self
            .session_ids
            .write()
            .ok()
            .and_then(|mut sessions| sessions.remove(session_id))
        else {
            return;
        };

        if let Ok(mut principals) = self.principals.write() {
            let remove_principal = if let Some(sessions) = principals.get_mut(info.principal()) {
                sessions.remove(session_id);
                sessions.is_empty()
            } else {
                false
            };
            if remove_principal {
                principals.remove(info.principal());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::session::{
        session_events::{SessionDestroyedEvent, SessionEvent, SessionIdChangedEvent},
        session_registry::SessionRegistry,
        session_registry_impl::SessionRegistryImpl,
    };

    #[test]
    fn registry_registers_lists_and_removes_sessions() {
        let registry = SessionRegistryImpl::new();
        registry.register_new_session("s1", "alice");
        registry.register_new_session("s2", "alice");

        assert_eq!(registry.all_principals(), vec![String::from("alice")]);
        assert_eq!(registry.all_sessions("alice", false).len(), 2);
        assert!(registry.session_information("s1").is_some());

        registry.remove_session_information("s1");

        assert!(registry.session_information("s1").is_none());
        assert_eq!(registry.all_sessions("alice", false).len(), 1);
    }

    #[test]
    fn registry_replaces_existing_session_id_registration() {
        let registry = SessionRegistryImpl::new();
        registry.register_new_session("s1", "alice");
        registry.register_new_session("s1", "bob");

        assert!(registry.all_sessions("alice", true).is_empty());
        assert_eq!(registry.all_sessions("bob", true).len(), 1);
    }

    #[test]
    fn registry_handles_destroyed_and_id_changed_events() {
        let registry = SessionRegistryImpl::new();
        registry.register_new_session("s1", "alice");
        registry.on_application_event(SessionEvent::IdChanged(SessionIdChangedEvent::new(
            "source", "s1", "s2",
        )));

        assert!(registry.session_information("s1").is_none());
        assert!(registry.session_information("s2").is_some());

        registry.on_application_event(SessionEvent::Destroyed(SessionDestroyedEvent::new(
            "source",
            "s2",
            Vec::new(),
        )));

        assert!(registry.session_information("s2").is_none());
        assert!(registry.all_principals().is_empty());
    }
}
