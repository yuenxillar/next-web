use std::sync::Arc;

use next_web_core::async_trait;
use tracing::debug;

use crate::core::session::{
    mgt::eis::session_dao::SessionDAO, Session, SessionError, SessionId, SessionValue,
};

/// A session that reads the session it stands for out of the session store on
/// every call.
///
/// Apache Shiro hands an application a session like this one: the session the
/// store holds can be replaced, stopped, or expired between two requests, and
/// every call has to see what the store holds now instead of what it held when
/// the handle was taken. The identifier is the only thing the handle keeps.
#[derive(Clone)]
pub struct DelegatingSession {
    session_dao: Arc<dyn SessionDAO>,
    id: SessionId,
}

impl DelegatingSession {
    /// Creates a handle for the session with the given identifier.
    ///
    /// # Arguments
    ///
    /// * `session_dao` - The store the session is read from.
    /// * `id` - The identifier of the session.
    pub fn new(session_dao: Arc<dyn SessionDAO>, id: SessionId) -> Self {
        Self { session_dao, id }
    }

    /// Returns the store the session is read from.
    pub fn session_dao(&self) -> &Arc<dyn SessionDAO> {
        &self.session_dao
    }

    /// Returns the session the store holds, or the error of a session that is
    /// no longer there.
    async fn do_get_session(&self) -> Result<Arc<dyn Session>, SessionError> {
        match self.session_dao.read(&self.id).await {
            Some(session) => Ok(session),
            None => {
                debug!("There is no session with id [{}].", self.id);

                Err(SessionError::NotFound)
            }
        }
    }
}

#[async_trait]
impl Session for DelegatingSession {
    fn id(&self) -> &SessionId {
        &self.id
    }

    async fn start_timestamp(&self) -> i64 {
        match self.do_get_session().await {
            Ok(session) => session.start_timestamp().await,
            Err(_) => 0,
        }
    }

    async fn last_access_time(&self) -> Option<i64> {
        match self.do_get_session().await {
            Ok(session) => session.last_access_time().await,
            Err(_) => None,
        }
    }

    async fn timeout(&self) -> Result<i64, SessionError> {
        self.do_get_session().await?.timeout().await
    }

    async fn set_timeout(&self, max_idle_time_in_millis: i64) -> Result<(), SessionError> {
        self.do_get_session()
            .await?
            .set_timeout(max_idle_time_in_millis)
            .await
    }

    async fn host(&self) -> Option<String> {
        match self.do_get_session().await {
            Ok(session) => session.host().await,
            Err(_) => None,
        }
    }

    async fn touch(&self) -> Result<(), SessionError> {
        self.do_get_session().await?.touch().await
    }

    async fn stop(&self) -> Result<(), SessionError> {
        self.do_get_session().await?.stop().await
    }

    async fn attribute_keys(&self) -> Result<Vec<String>, SessionError> {
        self.do_get_session().await?.attribute_keys().await
    }

    async fn get_attribute(&self, key: &str) -> Option<SessionValue> {
        match self.do_get_session().await {
            Ok(session) => session.get_attribute(key).await,
            Err(_) => None,
        }
    }

    async fn set_attribute(&self, key: &str, value: SessionValue) -> Result<(), SessionError> {
        self.do_get_session().await?.set_attribute(key, value).await
    }

    async fn remove_attribute(&self, key: &str) -> Result<Option<SessionValue>, SessionError> {
        self.do_get_session().await?.remove_attribute(key).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::DelegatingSession;
    use crate::core::session::{
        mgt::{
            eis::{memory_session_dao::MemorySessionDAO, session_dao::SessionDAO},
            simple_session::SimpleSession,
        },
        Session, SessionError, SessionId, SessionValue,
    };

    async fn store(session: &Arc<dyn Session>) -> Arc<MemorySessionDAO> {
        let dao = Arc::new(MemorySessionDAO::default());
        dao.create(session.clone()).await;

        dao
    }

    #[tokio::test]
    async fn the_handle_reads_the_session_the_store_holds() {
        let session: Arc<dyn Session> = Arc::new(SimpleSession::new("example.org"));
        let dao = store(&session).await;
        let delegating = DelegatingSession::new(dao, session.id().clone());

        assert_eq!(delegating.id(), session.id());
        assert_eq!(
            delegating.start_timestamp().await,
            session.start_timestamp().await
        );
        assert_eq!(delegating.host().await.as_deref(), Some("example.org"));
    }

    #[tokio::test]
    async fn an_attribute_written_through_the_handle_is_read_back() {
        let session: Arc<dyn Session> = Arc::new(SimpleSession::new("example.org"));
        let dao = store(&session).await;
        let delegating = DelegatingSession::new(dao, session.id().clone());

        delegating
            .set_attribute("authenticated", SessionValue::Boolean(true))
            .await
            .expect("the attribute is written");

        assert_eq!(
            delegating
                .get_attribute("authenticated")
                .await
                .and_then(|value| value.as_boolean()),
            Some(true)
        );
        assert_eq!(delegating.attribute_keys().await.map(|keys| keys.len()), Ok(1));
    }

    #[tokio::test]
    async fn a_stopped_session_is_what_the_next_call_sees() {
        let session: Arc<dyn Session> = Arc::new(SimpleSession::new("example.org"));
        let dao = store(&session).await;
        let delegating = DelegatingSession::new(dao, session.id().clone());

        delegating.stop().await.expect("the session stops");

        assert!(session.as_ref().timeout().await.is_ok());
    }

    #[tokio::test]
    async fn a_session_the_store_does_not_hold_reports_not_found() {
        let session: Arc<dyn Session> = Arc::new(SimpleSession::new("example.org"));
        let dao = store(&session).await;
        let delegating =
            DelegatingSession::new(dao, SessionId::String("no-such-session".to_string()));

        assert!(matches!(
            delegating.timeout().await,
            Err(SessionError::NotFound)
        ));
        assert!(delegating.get_attribute("any").await.is_none());
    }
}
