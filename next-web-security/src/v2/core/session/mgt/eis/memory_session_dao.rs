use std::sync::Arc;

use dashmap::DashMap;
use next_web_core::async_trait;

use crate::core::session::{mgt::eis::session_dao::SessionDAO, Session, SessionId};

/// A session store that keeps the sessions of an application in memory.
///
/// It is the store of a single process: a restart forgets every session, and a
/// deployment of several processes needs a store they share.
#[derive(Clone, Default)]
pub struct MemorySessionDAO {
    sessions: Arc<DashMap<String, Arc<dyn Session>>>,
}

impl MemorySessionDAO {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of sessions the store holds.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Returns whether the store holds no session at all.
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

#[async_trait]
impl SessionDAO for MemorySessionDAO {
    async fn create(&self, session: Arc<dyn Session>) {
        self.sessions.insert(session.id().to_string(), session);
    }

    async fn read(&self, session_id: &SessionId) -> Option<Arc<dyn Session>> {
        self.sessions
            .get(&session_id.to_string())
            .map(|session| session.value().clone())
    }

    async fn update(&self, session: Arc<dyn Session>) -> Result<(), &'static str> {
        self.sessions.insert(session.id().to_string(), session);

        Ok(())
    }

    async fn delete(&self, session: &dyn Session) {
        self.sessions.remove(&session.id().to_string());
    }

    async fn get_active_sessions(&self) -> Vec<Arc<dyn Session>> {
        self.sessions
            .iter()
            .map(|session| session.value().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::MemorySessionDAO;
    use crate::core::session::{
        mgt::{eis::session_dao::SessionDAO, simple_session::SimpleSession},
        Session, SessionId, SessionValue,
    };

    fn session() -> Arc<dyn Session> {
        Arc::new(SimpleSession::new("example.org"))
    }

    #[tokio::test]
    async fn a_created_session_is_readable_by_its_identifier() {
        let dao = MemorySessionDAO::default();
        let session = session();
        dao.create(session.clone()).await;

        let read = dao.read(session.id()).await.expect("the session is stored");
        assert_eq!(read.id(), session.id());
        assert_eq!(dao.len(), 1);
    }

    #[tokio::test]
    async fn a_session_that_was_never_stored_is_not_readable() {
        let dao = MemorySessionDAO::default();

        assert!(dao
            .read(&SessionId::String("no-such-session".to_string()))
            .await
            .is_none());
        assert!(dao.is_empty());
    }

    #[tokio::test]
    async fn a_deleted_session_is_gone_and_the_others_stay() {
        let dao = MemorySessionDAO::default();
        let first = session();
        let second = session();
        dao.create(first.clone()).await;
        dao.create(second.clone()).await;

        dao.delete(first.as_ref()).await;

        assert!(dao.read(first.id()).await.is_none());
        assert!(dao.read(second.id()).await.is_some());
        assert_eq!(dao.get_active_sessions().await.len(), 1);
    }

    #[tokio::test]
    async fn an_updated_session_is_read_back_with_its_state() {
        let dao = MemorySessionDAO::default();
        let session = session();
        dao.create(session.clone()).await;

        session
            .set_attribute("k", SessionValue::Boolean(true))
            .await
            .expect("the attribute is set");
        dao.update(session.clone()).await.expect("the session updates");

        let read = dao.read(session.id()).await.expect("the session is stored");
        assert_eq!(
            read.get_attribute("k")
                .await
                .and_then(|value| value.as_boolean()),
            Some(true)
        );
    }
}
