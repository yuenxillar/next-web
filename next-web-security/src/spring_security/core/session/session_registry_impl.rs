use std::{
    any::Any,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use chrono::Utc;
use next_web_context::{ApplicationEvent, ApplicationListener};
use next_web_core::async_trait;
use tokio::sync::RwLock;
use tracing::{enabled, Level};

use super::{
    session_events::SessionEvent, session_information::SessionInformation,
    session_registry::SessionRegistry,
};

#[derive(Clone)]
pub struct SessionRegistryImpl {
    principals: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    session_ids: Arc<RwLock<HashMap<String, SessionInformation>>>,
}

impl SessionRegistryImpl {
    pub fn new(
        principals: HashMap<String, HashSet<String>>,
        session_ids: HashMap<String, SessionInformation>,
    ) -> Self {
        Self {
            principals: Arc::new(RwLock::new(principals)),
            session_ids: Arc::new(RwLock::new(session_ids)),
        }
    }

    async fn with_session_mut<F, R>(&self, session_id: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut SessionInformation) -> R,
    {
        assert!(
            !session_id.trim().is_empty(),
            "SessionId required as per interface contract"
        );

        let mut sessions = self.session_ids.write().await;
        sessions.get_mut(session_id).map(f)
    }
}

impl ApplicationListener<Box<dyn ApplicationEvent>> for SessionRegistryImpl {
    fn on_application_event(&self, event: Box<dyn ApplicationEvent>) {
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
        todo!()
    }
}

#[async_trait]
impl SessionRegistry for SessionRegistryImpl {
    async fn all_principals(&self) -> Vec<String> {
        let mut principals = self
            .principals
            .read()
            .map(|principals| principals.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        principals.sort();
        principals
    }

    async fn all_sessions(
        &self,
        principal: &dyn Any,
        include_expired_sessions: bool,
    ) -> Vec<SessionInformation> {
        let session_ids = self
            .principals
            .read()
            .await
            .get(principal)
            .cloned()
            .unwrap_or_default();

        session_ids
            .into_iter()
            .filter_map(|session_id| self.session_information(&session_id))
            .filter(|info| include_expired_sessions || !info.is_expired())
            .collect()
    }

    async fn session_information<'a>(&'a self, session_id: &str) -> Option<&'a SessionInformation> {
        assert!(
            !session_id.is_empty(),
            "SessionId required as per interface contract"
        );
        self.session_ids.read().await.get(session_id)
    }

    async fn refresh_last_request(&self, session_id: &str) {
        assert!(
            !session_id.is_empty(),
            "SessionId required as per interface contract"
        );

        self.with_session_mut(session_id, |info| {
            info.refresh_last_request();
        })
        .await;
    }

    async fn register_new_session(&self, session_id: &str, principal: &dyn Any) {
        let session_id = session_id.to_string();
        assert!(
            !session_id.is_empty(),
            "SessionId required as per interface contract"
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

    async fn remove_session_information(&self, session_id: &str) {
        assert!(
            !session_id.is_empty(),
            "SessionId required as per interface contract"
        );

        let info = match self.session_information(session_id) {
            Some(info) => info,
            None => return,
        };

        self.session_ids.write().await.remove(session_id);

        if enabled!(Level::TRACE) {
            tracing::trace!(
                "Removing session {} from set of registered sessions",
                session_id
            );
        }
    }
}

impl Default for SessionRegistryImpl {
    fn default() -> Self {
        Self {
            principals: Arc::new(RwLock::new(Default::default())),
            session_ids: Arc::new(RwLock::new(Default::default())),
        }
    }
}
