use std::{
    any::Any,
    collections::{HashMap, HashSet},
    sync::Arc,
    time::SystemTime,
};

use next_web_context::{ApplicationEvent, ApplicationListener};
use next_web_core::{async_trait, util::StringUtils, BoxFuture};
use tokio::sync::RwLock;
use tracing::{debug, enabled, trace, Level};

use crate::web::authentication::AuthPrincipal;

use super::{session_event::SessionEvent, session_registry::SessionRegistry, SessionInformation};

/// Default implementation of `SessionRegistry` which listens for session
/// events published in the application context.
///
/// Principals are indexed by their string representation, so the principal
/// must be representable as a `String`.
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

    async fn get_session_information(&self, session_id: &str) -> Option<SessionInformation> {
        self.session_ids.read().await.get(session_id).cloned()
    }

    async fn has_session_information(&self, session_id: &str) -> bool {
        self.session_ids.read().await.contains_key(session_id)
    }
}

impl ApplicationListener<Box<dyn ApplicationEvent>> for SessionRegistryImpl {
    fn on_application_event<'a>(&'a self, event: Box<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let session_event = match (event.as_ref() as &dyn Any).downcast_ref::<SessionEvent>() {
                Some(event) => event,
                None => return,
            };
            match session_event {
                SessionEvent::Destroyed(session_destroyed_event) => {
                    self.remove_session_information(session_destroyed_event.id())
                        .await;
                }
                SessionEvent::IdChanged(session_id_changed_event) => {
                    if let Some(info) = self
                        .session_ids
                        .read()
                        .await
                        .get(session_id_changed_event.old_session_id())
                    {
                        self.remove_session_information(session_id_changed_event.old_session_id())
                            .await;
                        self.register_new_session(
                            session_id_changed_event.new_session_id(),
                            info.principal(),
                        )
                        .await;
                    }
                }
                SessionEvent::Created(_) => {}
            }
        })
    }
}

#[async_trait]
impl SessionRegistry for SessionRegistryImpl {
    async fn all_principals(&self) -> Vec<String> {
        self.principals.read().await.keys().cloned().collect()
    }

    async fn all_sessions(
        &self,
        principal: &AuthPrincipal,
        include_expired_sessions: bool,
    ) -> Vec<SessionInformation> {
        let principal_key = principal.to_string();

        let mut sessions = Vec::new();
        if let Some(sessions_used_by_principal) = self.principals.read().await.get(&principal_key) {
            if !sessions_used_by_principal.is_empty() {
                sessions.reserve(sessions_used_by_principal.len());
            }

            for session_id in sessions_used_by_principal {
                if let Some(info) = self.session_information(&session_id).await {
                    if include_expired_sessions || !info.is_expired() {
                        sessions.push(info);
                    }
                }
            }
        }
        sessions
    }

    async fn session_information(&self, session_id: &str) -> Option<SessionInformation> {
        assert!(
            StringUtils::has_text(session_id),
            "SessionId required as per interface contract"
        );
        self.get_session_information(session_id).await
    }

    async fn refresh_last_request(&self, session_id: &str) {
        assert!(
            StringUtils::has_text(session_id),
            "SessionId required as per interface contract"
        );
        if let Some(info) = self.session_ids.write().await.get_mut(session_id) {
            info.refresh_last_request();
        }
    }

    async fn register_new_session(&self, session_id: &str, principal: &AuthPrincipal) {
        assert!(
            StringUtils::has_text(session_id),
            "SessionId required as per interface contract"
        );
        if self.has_session_information(session_id).await {
            self.remove_session_information(session_id).await;
        }

        let principal_key = principal.to_string();
        if enabled!(Level::DEBUG) {
            debug!(
                "Registering session {} for principal {}",
                session_id, principal_key
            );
        }

        self.session_ids.write().await.insert(
            session_id.to_string(),
            SessionInformation::new(principal.clone(), session_id.to_string(), SystemTime::now()),
        );
        self.principals
            .write()
            .await
            .entry(principal_key)
            .or_default()
            .insert(session_id.to_string());
    }

    async fn remove_session_information(&self, session_id: &str) {
        assert!(
            StringUtils::has_text(session_id),
            "SessionId required as per interface contract"
        );

        let info = match self.session_information(session_id).await {
            Some(info) => info,
            None => return,
        };
        if enabled!(Level::TRACE) {
            trace!(
                "Removing session {} from set of registered sessions",
                session_id
            );
        }
        self.session_ids.write().await.remove(session_id);

        let principal_key = info.principal().to_string();
        if let Some(sessions_used_by_principal) =
            self.principals.write().await.get_mut(&principal_key)
        {
            debug!(
                "Removing session {} from principal's set of registered sessions",
                session_id
            );
            sessions_used_by_principal.remove(session_id);
            if sessions_used_by_principal.is_empty() {
                if enabled!(Level::DEBUG) {
                    debug!("Removing principal {} from registry", principal_key);
                }
                sessions_used_by_principal.remove(&principal_key);
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn principal(name: &str) -> AuthPrincipal {
        Arc::new(name.to_string())
    }

    #[tokio::test]
    async fn register_new_session_and_query() {
        let registry = SessionRegistryImpl::default();
        let principal = principal("admin");
        registry.register_new_session("1", &principal).await;

        let info = registry
            .session_information("1")
            .await
            .expect("session 1 should be registered");
        assert_eq!(info.session_id(), "1");
        assert_eq!(
            info.principal()
                .as_any()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("admin")
        );
        assert!(!info.is_expired());

        let sessions = registry.all_sessions(&principal, false).await;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id(), "1");

        assert_eq!(registry.all_principals().await, vec!["admin".to_string()]);
    }

    #[tokio::test]
    async fn remove_session_information_cleans_up() {
        let registry = SessionRegistryImpl::default();
        let principal = principal("admin");
        registry.register_new_session("1", &principal).await;
        registry.register_new_session("2", &principal).await;

        registry.remove_session_information("1").await;
        assert!(registry.session_information("1").await.is_none());
        assert_eq!(registry.all_sessions(&principal, true).await.len(), 1);
        assert_eq!(registry.all_principals().await, vec!["admin".to_string()]);

        registry.remove_session_information("2").await;
        assert!(registry.all_sessions(&principal, true).await.is_empty());
        assert!(registry.all_principals().await.is_empty());
    }

    #[tokio::test]
    async fn refresh_last_request_updates_timestamp() {
        let registry = SessionRegistryImpl::default();
        let principal = principal("admin");
        registry.register_new_session("1", &principal).await;

        let before = registry
            .session_information("1")
            .await
            .expect("session registered")
            .last_request();
        std::thread::sleep(Duration::from_millis(5));
        registry.refresh_last_request("1").await;
        let after = registry
            .session_information("1")
            .await
            .expect("session registered")
            .last_request();
        assert!(after >= before);
    }

    #[tokio::test]
    async fn all_sessions_filters_expired() {
        let principal = principal("admin");
        let mut info = SessionInformation::new(principal.clone(), "1", SystemTime::now());
        info.expire_now();

        let mut principals = HashMap::new();
        principals.insert("admin".to_string(), HashSet::from(["1".to_string()]));
        let mut session_ids = HashMap::new();
        session_ids.insert("1".to_string(), info);
        let registry = SessionRegistryImpl::new(principals, session_ids);

        assert!(registry.all_sessions(&principal, false).await.is_empty());
        assert_eq!(registry.all_sessions(&principal, true).await.len(), 1);
    }
}
