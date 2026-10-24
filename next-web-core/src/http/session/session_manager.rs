//! The [`SessionManager`] contract.
//!
//! A session manager owns the live sessions of the container and is the single
//! place that creates, looks up, expires, and invalidates them. Implementations
//! live behind [`SessionManager`] so an application can swap the in-memory
//! [`DefaultSessionManager`](crate::http::session::DefaultSessionManager) for a
//! distributed store without changing its call sites.

use std::{
    fmt::Debug,
    sync::{Arc, Mutex, MutexGuard},
};

use uuid::Uuid;

use super::standard_session::current_time_millis;
use super::{HttpSession, StandardSession};

/// A cheap, cloneable handle to a live session.
///
/// Handles share the same [`StandardSession`], so a mutation performed through
/// one handle (or through the manager) is observed by every other handle. The
/// [`Mutex`] serializes that access and is what lets the manager offer `&self`
/// methods that still mutate sessions.
pub type SessionHandle = Arc<Mutex<StandardSession>>;

/// Notified when a session of the container is created, replaced or destroyed.
///
/// The manager of the sessions is the only place that knows when a session comes
/// into being and when it ends, so the parts of the application that have to react
/// to that (the registry of the sessions of an application, the audit of the
/// sessions, a cache that is keyed by a session) are told by the manager through
/// this seam. A listener does not decide anything: the session is created or
/// destroyed and the manager hands it the session that was.
pub trait SessionLifecycleListener
where
    Self: Send + Sync,
{
    /// A session was created under an identifier the store did not hold.
    ///
    /// # Arguments
    ///
    /// * `session` - The session that was created.
    fn session_created(&self, session: Arc<dyn HttpSession>);

    /// A session was destroyed.
    ///
    /// # Arguments
    ///
    /// * `session` - The session that was destroyed.
    fn session_destroyed(&self, session: Arc<dyn HttpSession>);
}

/// Owns and manages the lifetime of [`HttpSession`]s.
///
/// Implementations must be `Send + Sync + Debug` so the manager can be shared
/// across worker threads and stored in a trait object (`Arc<dyn
/// SessionManager>`).
///
/// The trait is split into a small set of required storage primitives and a
/// larger set of provided methods that are implemented on top of them. A store
/// only has to provide the primitives below; everything else works out of the
/// box.
///
/// # Required methods
///
/// * [`SessionManager::put_session`]
/// * [`SessionManager::get_session`]
/// * [`SessionManager::remove_session`]
/// * [`SessionManager::session_ids`]
/// * [`SessionManager::len`]
/// * [`SessionManager::default_max_inactive_interval`]
/// * [`SessionManager::set_default_max_inactive_interval`]
pub trait SessionManager: Send + Sync + Debug {
    // ------------------------------------------------------------------
    // Storage primitives.
    // ------------------------------------------------------------------

    /// Stores `session` (replacing any session that shares its identifier) and
    /// returns a handle to it.
    ///
    /// This is the write primitive used both when a session is created and when
    /// a working copy is saved back at the end of a request.
    fn put_session(&self, session: StandardSession) -> SessionHandle;

    /// Returns a handle to the session with the given id, if it exists.
    fn get_session(&self, id: &str) -> Option<SessionHandle>;

    /// Removes the session with the given id.
    ///
    /// Returns `true` if a session was removed. This does not call
    /// [`HttpSession::invalidate`]; use [`SessionManager::invalidate_session`]
    /// for the full invalidation workflow.
    fn remove_session(&self, id: &str) -> bool;

    /// Returns a snapshot of all live session identifiers.
    fn session_ids(&self) -> Vec<String>;

    /// Returns the number of live sessions.
    fn len(&self) -> usize;

    /// Returns the idle timeout, in seconds, applied to newly created sessions.
    ///
    /// A value of `0` disables the timeout.
    fn default_max_inactive_interval(&self) -> u64;

    /// Sets the idle timeout, in seconds, applied to newly created sessions.
    fn set_default_max_inactive_interval(&self, seconds: u64);

    // ------------------------------------------------------------------
    // Provided behavior.
    // ------------------------------------------------------------------

    /// Creates a new session with a freshly generated identifier.
    fn create_session(&self) -> SessionHandle {
        self.create_session_with_id(generate_session_id())
    }

    /// Creates a new empty session with the given identifier, registers it, and
    /// returns a handle to it.
    ///
    /// The idle timeout is initialized from
    /// [`SessionManager::default_max_inactive_interval`]. If a session with the
    /// same id already exists it is replaced.
    fn create_session_with_id(&self, id: String) -> SessionHandle {
        let mut session = StandardSession::new(id);
        session.set_max_inactive_interval(self.default_max_inactive_interval());
        self.put_session(session)
    }

    /// Returns an owned copy of the session with the given id.
    ///
    /// A request uses this to load its working copy; mutations to that copy are
    /// written back with [`SessionManager::put_session`].
    fn load_session(&self, id: &str) -> Option<StandardSession> {
        self.get_session(id).map(|handle| lock(&handle).clone())
    }

    /// Returns `true` when a session with the given id exists.
    fn contains_session(&self, id: &str) -> bool {
        self.get_session(id).is_some()
    }

    /// Returns `true` when the manager holds no sessions.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Removes every session.
    fn clear(&self) {
        for id in self.session_ids() {
            self.remove_session(&id);
        }
    }

    /// Invalidates and removes the session with the given id.
    ///
    /// Existing handles observe an invalidated session, and the session is
    /// unregistered so it can no longer be looked up. Returns `true` if a
    /// session was present.
    fn invalidate_session(&self, id: &str) -> bool {
        if let Some(handle) = self.get_session(id) {
            lock(&handle).invalidate();
        }
        self.remove_session(id)
    }

    /// Refreshes the last-accessed time of the session with the given id.
    ///
    /// Returns `true` if the session exists.
    fn touch_session(&self, id: &str) -> bool {
        match self.get_session(id) {
            Some(handle) => {
                lock(&handle).touch();
                true
            }
            None => false,
        }
    }

    /// Runs `f` with a shared view of the session with the given id.
    ///
    /// The session lock is held for the duration of the call, so `f` must not
    /// call back into the manager for the same session. Returns `true` if the
    /// session existed and `f` ran.
    fn with_session(&self, id: &str, f: &mut dyn FnMut(&dyn HttpSession)) -> bool {
        match self.get_session(id) {
            Some(handle) => {
                let guard = lock(&handle);
                f(&*guard);
                true
            }
            None => false,
        }
    }

    /// Removes every session whose idle timeout has elapsed.
    ///
    /// Returns the number of sessions that were removed.
    fn expire_idle_sessions(&self) -> usize {
        let now = current_time_millis();
        let mut removed = 0;

        for id in self.session_ids() {
            let expired = match self.get_session(&id) {
                Some(handle) => lock(&handle).is_expired(now),
                None => continue,
            };
            if expired && self.remove_session(&id) {
                removed += 1;
            }
        }

        removed
    }

    /// Assigns a fresh identifier to an existing session.
    ///
    /// The session state (attributes, timestamps, flags, and timeout) is
    /// migrated onto the new identifier and the old identifier is removed. This
    /// is the standard defence against session-fixation attacks.
    ///
    /// Returns the new identifier, or `None` if the session does not exist.
    fn change_session_id(&self, old_id: &str) -> Option<String> {
        let old_handle = self.get_session(old_id)?;
        let new_id = generate_session_id();
        let new_handle = self.create_session_with_id(new_id.clone());

        {
            let old_guard = lock(&old_handle);
            let mut new_guard = lock(&new_handle);
            new_guard.copy_state_from(&old_guard);
        }

        self.remove_session(old_id);
        Some(new_id)
    }
}

/// Generates a new, unpredictable session identifier.
///
/// The identifier is a 128-bit UUID v4 rendered without hyphens, which yields a
/// 32-character, cookie-safe string.
#[must_use]
pub fn generate_session_id() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Locks a session handle, recovering the inner value if the mutex was poisoned
/// by a panic in another thread.
///
/// A poisoned session is still usable: every mutation is a plain field update,
/// so there is no partially applied invariant to protect.
pub(crate) fn lock(session: &SessionHandle) -> MutexGuard<'_, StandardSession> {
    session
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
