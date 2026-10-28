use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::session::{Session, SessionId};

/// The store the sessions of an application are kept in.
///
/// A session manager reads and writes through this trait, so an application can
/// keep its sessions where the next request finds them: in memory, in a shared
/// cache, or in a database. The store owns the handles it returns, because the
/// state of a session can change between two calls.
#[async_trait]
pub trait SessionDAO
where
    Self: Send + Sync,
{
    /// Stores a session.
    ///
    /// # Arguments
    ///
    /// * `session` - The session to store.
    async fn create(&self, session: Arc<dyn Session>);

    /// Returns the session with the given identifier, when the store holds it.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The identifier of the session.
    async fn read(&self, session_id: &SessionId) -> Option<Arc<dyn Session>>;

    /// Stores the state of a session the store already holds.
    ///
    /// # Arguments
    ///
    /// * `session` - The session with the state to store.
    async fn update(&self, session: Arc<dyn Session>) -> Result<(), &'static str>;

    /// Removes a session.
    ///
    /// # Arguments
    ///
    /// * `session` - The session to remove.
    async fn delete(&self, session: &dyn Session);

    /// Returns every session the store holds.
    async fn get_active_sessions(&self) -> Vec<Arc<dyn Session>>;
}
