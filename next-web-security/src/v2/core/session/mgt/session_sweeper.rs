//! Sweeping a session store without holding the manager that owns it.
//!
//! A validating session manager sweeps its own store, which is what a request
//! path does. A scheduler, on the other hand, has to sweep the store of a manager
//! that owns the scheduler — a circular reference the Java version resolves with a
//! back-pointer. The port sidesteps it: a sweeper holds the store itself, so a
//! scheduler can be built from it before the manager that uses the same store.

use std::any::Any;
use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::session::{
    mgt::{
        eis::session_dao::SessionDAO,
        simple_session::SimpleSession,
        validating_session::ValidatingSession,
        validating_session_manager::ValidatingSessionManager,
    },
};

/// A session store that can be swept for expired sessions.
///
/// It is the sweep of [`ValidatingSessionManager`] without the manager: the same
/// session store, the same rule about dropping an expired session, and no
/// reference back to the object that holds the scheduler.
#[derive(Clone)]
pub struct SessionSweeper {
    session_dao: Arc<dyn SessionDAO>,
    delete_invalid_sessions: bool,
}

impl SessionSweeper {
    /// Creates a sweeper over the given store.
    ///
    /// # Arguments
    ///
    /// * `session_dao` - The store that is swept.
    pub fn new(session_dao: Arc<dyn SessionDAO>) -> Self {
        Self {
            session_dao,
            // A session that expired is of no use to anyone: dropping it keeps the
            // store from growing with the sessions nobody will present again.
            delete_invalid_sessions: true,
        }
    }

    /// Sets whether an expired session is dropped from the store.
    ///
    /// # Arguments
    ///
    /// * `delete_invalid_sessions` - Whether the sweep deletes an expired session.
    pub fn with_delete_invalid_sessions(mut self, delete_invalid_sessions: bool) -> Self {
        self.delete_invalid_sessions = delete_invalid_sessions;
        self
    }

    /// Returns the store that is swept.
    pub fn session_dao(&self) -> &Arc<dyn SessionDAO> {
        &self.session_dao
    }

    /// Returns whether an expired session is dropped from the store.
    pub fn is_delete_invalid_sessions(&self) -> bool {
        self.delete_invalid_sessions
    }
}

#[async_trait]
impl ValidatingSessionManager for SessionSweeper {
    async fn validate_sessions(&self) {
        for session in self.session_dao.get_active_sessions().await {
            let Some(simple_session) =
                (session.as_ref() as &dyn Any).downcast_ref::<SimpleSession>()
            else {
                continue;
            };

            if simple_session.validate().is_ok() {
                continue;
            }

            simple_session.set_expired(true);
            self.session_dao.update(session.clone()).await.ok();

            if self.delete_invalid_sessions {
                self.session_dao.delete(session.as_ref()).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::sync::Arc;

    use super::SessionSweeper;
    use crate::core::session::{
        mgt::{
            eis::{memory_session_dao::MemorySessionDAO, session_dao::SessionDAO},
            simple_session::SimpleSession,
            validating_session_manager::ValidatingSessionManager,
        },
        Session,
    };


    #[tokio::test]
    async fn the_sweep_drops_the_sessions_that_expired() {
        let dao = MemorySessionDAO::default();
        let sweeper = SessionSweeper::new(Arc::new(dao.clone()));

        let live: Arc<dyn Session> = Arc::new(SimpleSession::new("example.org"));
        dao.create(live.clone()).await;
        let expired: Arc<dyn Session> = Arc::new(SimpleSession::new("example.org"));
        dao.create(expired.clone()).await;
        if let Some(session) = (expired.as_ref() as &dyn Any).downcast_ref::<SimpleSession>() {
            session.expire();
        }

        sweeper.validate_sessions().await;

        assert!(dao.read(live.id()).await.is_some());
        assert!(dao.read(expired.id()).await.is_none());
        assert!(sweeper.is_delete_invalid_sessions());
        assert_eq!(sweeper.session_dao().get_active_sessions().await.len(), 1);
    }

    #[tokio::test]
    async fn a_sweeper_that_keeps_the_sessions_keeps_the_expired_one() {
        let dao = MemorySessionDAO::default();
        let sweeper = SessionSweeper::new(Arc::new(dao.clone())).with_delete_invalid_sessions(false);

        let expired: Arc<dyn Session> = Arc::new(SimpleSession::new("example.org"));
        dao.create(expired.clone()).await;
        if let Some(session) = (expired.as_ref() as &dyn Any).downcast_ref::<SimpleSession>() {
            session.expire();
        }

        sweeper.validate_sessions().await;

        assert!(dao.read(expired.id()).await.is_some());
        assert!(!sweeper.is_delete_invalid_sessions());
    }

    #[tokio::test]
    async fn the_sweeper_can_be_the_manager_of_a_scheduler() {
        use crate::core::session::mgt::{
            executor_service_session_validation_scheduler::ExecutorServiceSessionValidationScheduler,
            session_validation_scheduler::SessionValidationScheduler,
        };

        let dao = MemorySessionDAO::default();
        let session: Arc<dyn Session> = Arc::new(SimpleSession::new("example.org"));
        dao.create(session.clone()).await;
        if let Some(simple) = (session.as_ref() as &dyn Any).downcast_ref::<SimpleSession>() {
            simple.expire();
        }

        // The scheduler is built from the store, not from the manager that owns the
        // store, which is what keeps the two from pointing at each other.
        let sweeper = Arc::new(SessionSweeper::new(Arc::new(dao.clone())));
        let mut scheduler = ExecutorServiceSessionValidationScheduler::with_interval(
            sweeper,
            std::time::Duration::from_millis(5),
        );

        scheduler.enable_session_validation();
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        scheduler.disable_session_validation();

        assert!(dao.read(session.id()).await.is_none());
    }

}
