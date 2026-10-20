//! The in-memory [`SessionManager`] implementation.

use std::{
    fmt,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock, RwLock, Weak,
    },
};

use dashmap::DashMap;

use super::{HttpSession, SessionHandle, SessionLifecycleListener, SessionManager, StandardSession};

/// An in-memory [`SessionManager`] backed by a sharded concurrent map.
///
/// Sessions are stored keyed by identifier and handed out as
/// [`SessionHandle`]s, so callers observe a consistent, shared view of a
/// session across requests. New sessions get an idle timeout from the manager
/// default, which can be changed at any time (it affects sessions created
/// afterwards).
///
/// # Building a manager
///
/// [`DefaultSessionManager::new`] builds a plain manager. To enable
/// [`HttpSession::accessor`](crate::http::session::HttpSession::accessor),
/// create it as an `Arc` and call [`DefaultSessionManager::init`]:
///
/// ```
/// use std::sync::Arc;
///
/// use next_web_core::http::session::{DefaultSessionManager, HttpSession, SessionManager};
///
/// let manager: Arc<DefaultSessionManager> = DefaultSessionManager::shared();
/// let session = manager.create_session_with_id("session-1".to_string());
///
/// let accessor = session.lock().unwrap().accessor().expect("accessor");
/// accessor.access(Box::new(|session| {
///     assert_eq!(session.id(), "session-1");
/// }));
/// ```
pub struct DefaultSessionManager {
    /// The live sessions, keyed by identifier.
    sessions: DashMap<String, SessionHandle>,
    /// Idle timeout applied to newly created sessions.
    default_max_inactive_interval: AtomicU64,
    /// Weak self-reference used to attach itself to created sessions so they
    /// can build accessors. Set by [`DefaultSessionManager::init`].
    manager: OnceLock<Weak<dyn SessionManager>>,
    /// The listeners that are told when a session is created or destroyed.
    lifecycle_listeners: RwLock<Vec<Arc<dyn SessionLifecycleListener>>>,
}

impl DefaultSessionManager {
    /// The default idle timeout, in seconds (30 minutes).
    pub const DEFAULT_MAX_INACTIVE_INTERVAL: u64 = 30 * 60;

    /// Creates an empty manager with the default idle timeout.
    ///
    /// Note that a manager created this way cannot attach to its sessions for
    /// [`HttpSession::accessor`](crate::http::session::HttpSession::accessor);
    /// call [`DefaultSessionManager::init`] on an `Arc` (or use
    /// [`DefaultSessionManager::shared`]) to enable that.
    #[must_use]
    pub fn new() -> Self {
        Self::with_max_inactive_interval(Self::DEFAULT_MAX_INACTIVE_INTERVAL)
    }

    /// Creates an empty manager that applies `seconds` as the idle timeout of
    /// newly created sessions.
    ///
    /// Pass `0` to disable the timeout.
    #[must_use]
    pub fn with_max_inactive_interval(seconds: u64) -> Self {
        Self {
            sessions: DashMap::new(),
            default_max_inactive_interval: AtomicU64::new(seconds),
            manager: OnceLock::new(),
            lifecycle_listeners: RwLock::new(Vec::new()),
        }
    }

    /// Adds a listener that is told when a session is created or destroyed.
    ///
    /// The listener is told for the sessions the manager creates from now on, and
    /// for the ones it destroys, which is what lets the parts of the application
    /// that follow the sessions of the container react to them.
    ///
    /// # Arguments
    ///
    /// * `listener` - The listener of the life of a session.
    pub fn add_lifecycle_listener(&self, listener: Arc<dyn SessionLifecycleListener>) {
        self.lifecycle_listeners
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(listener);
    }

    /// Returns the listeners of the life of a session.
    pub fn lifecycle_listeners(&self) -> Vec<Arc<dyn SessionLifecycleListener>> {
        self.lifecycle_listeners
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Tells the listeners of the creation of the session, when it is new.
    ///
    /// # Arguments
    ///
    /// * `session` - The session that was stored.
    /// * `created` - Whether the store held no session with that identifier.
    fn notify_created(&self, session: &StandardSession, created: bool) {
        if !created {
            return;
        }

        let session: Arc<dyn HttpSession> = Arc::new(session.clone());
        for listener in self.lifecycle_listeners() {
            listener.session_created(Arc::clone(&session));
        }
    }

    /// Tells the listeners of the destruction of the session.
    ///
    /// # Arguments
    ///
    /// * `session` - The session that was removed.
    fn notify_destroyed(&self, session: &StandardSession) {
        let session: Arc<dyn HttpSession> = Arc::new(session.clone());
        for listener in self.lifecycle_listeners() {
            listener.session_destroyed(Arc::clone(&session));
        }
    }

    /// Creates a shared, accessor-enabled manager.
    ///
    /// Equivalent to `Arc::new(DefaultSessionManager::new())` followed by
    /// [`DefaultSessionManager::init`].
    #[must_use]
    pub fn shared() -> Arc<Self> {
        let manager = Arc::new(Self::new());
        manager.init();
        manager
    }

    /// Attaches a weak self-reference so created sessions can vend an
    /// [`HttpSessionAccessor`](crate::http::session::HttpSessionAccessor).
    ///
    /// Calling this more than once has no effect. It must be called on an
    /// `Arc<DefaultSessionManager>`; sessions created before it runs (or
    /// without it) have no accessor.
    pub fn init(self: &Arc<Self>) {
        let concrete: Arc<Self> = Arc::clone(self);
        let shared: Arc<dyn SessionManager> = concrete;
        let _ = self.manager.set(Arc::downgrade(&shared));
    }
}

impl SessionManager for DefaultSessionManager {
    fn put_session(&self, mut session: StandardSession) -> SessionHandle {
        if let Some(manager) = self.manager.get() {
            session.attach_manager(manager.clone());
        }

        let session_id = session.id().to_string();
        let created = !self.sessions.contains_key(&session_id);
        let handle = Arc::new(Mutex::new(session.clone()));
        self.sessions.insert(session_id, Arc::clone(&handle));
        self.notify_created(&session, created);

        handle
    }

    fn get_session(&self, id: &str) -> Option<SessionHandle> {
        self.sessions.get(id).map(|entry| Arc::clone(entry.value()))
    }

    fn remove_session(&self, id: &str) -> bool {
        let Some((_, handle)) = self.sessions.remove(id) else {
            return false;
        };

        // The listeners are told with the session that ended, which is the one the
        // store held when it was removed.
        let session = handle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        self.notify_destroyed(&session);

        true
    }

    fn session_ids(&self) -> Vec<String> {
        self.sessions
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    fn len(&self) -> usize {
        self.sessions.len()
    }

    fn default_max_inactive_interval(&self) -> u64 {
        self.default_max_inactive_interval.load(Ordering::Relaxed)
    }

    fn set_default_max_inactive_interval(&self, seconds: u64) {
        self.default_max_inactive_interval
            .store(seconds, Ordering::Relaxed);
    }
}

impl Default for DefaultSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DefaultSessionManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultSessionManager")
            .field("sessions", &self.sessions.len())
            .field(
                "default_max_inactive_interval",
                &self.default_max_inactive_interval.load(Ordering::Relaxed),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::DefaultSessionManager;
    use crate::http::session::{HttpSession, SessionManager};

    /// A listener of the life of a session that records what it is told.
    #[derive(Default)]
    struct RecordingLifecycleListener {
        /// The identifiers of the sessions that were created.
        created: Mutex<Vec<String>>,
        /// The identifiers of the sessions that were destroyed.
        destroyed: Mutex<Vec<String>>,
    }

    impl crate::http::session::SessionLifecycleListener for RecordingLifecycleListener {
        fn session_created(&self, session: Arc<dyn HttpSession>) {
            self.created
                .lock()
                .unwrap()
                .push(session.id().to_string());
        }

        fn session_destroyed(&self, session: Arc<dyn HttpSession>) {
            self.destroyed
                .lock()
                .unwrap()
                .push(session.id().to_string());
        }
    }

    #[test]
    fn the_listeners_are_told_when_a_session_is_created_and_destroyed() {
        let manager = DefaultSessionManager::new();
        let listener = Arc::new(RecordingLifecycleListener::default());
        manager.add_lifecycle_listener(listener.clone());

        manager.create_session_with_id("s1".to_string());
        manager.create_session_with_id("s2".to_string());
        // Saving a session that is already stored is not a creation.
        manager.create_session_with_id("s1".to_string());
        manager.remove_session(&"s1".to_string());
        // Removing a session that is not stored tells the listeners nothing.
        manager.remove_session(&"gone".to_string());

        assert_eq!(
            *listener.created.lock().unwrap(),
            vec!["s1".to_string(), "s2".to_string()]
        );
        assert_eq!(*listener.destroyed.lock().unwrap(), vec!["s1".to_string()]);
    }
    #[test]
    fn sessions_can_be_created_listed_and_removed() {
        let manager = DefaultSessionManager::new();
        assert!(manager.is_empty());

        let handle = manager.create_session();
        let id = handle.lock().unwrap().id().to_string();

        assert_eq!(manager.len(), 1);
        assert!(manager.contains_session(&id));
        assert_eq!(manager.session_ids(), vec![id.clone()]);
        assert!(manager.remove_session(&id));
        assert!(manager.is_empty());
    }

    #[test]
    fn new_sessions_use_the_default_idle_timeout() {
        let manager = DefaultSessionManager::with_max_inactive_interval(120);
        let handle = manager.create_session_with_id("s1".to_string());

        assert_eq!(handle.lock().unwrap().max_inactive_interval(), 120);
    }

    #[test]
    fn idle_sessions_are_expired_and_removed() {
        let manager = DefaultSessionManager::with_max_inactive_interval(30);
        let stale = manager.create_session_with_id("stale".to_string());
        let active = manager.create_session_with_id("active".to_string());

        // Force the first session far into the past; the second stays fresh.
        stale.lock().unwrap().set_last_accessed_time(0);
        active.lock().unwrap().touch();

        assert_eq!(manager.expire_idle_sessions(), 1);
        assert!(!manager.contains_session("stale"));
        assert!(manager.contains_session("active"));
    }

    #[test]
    fn changing_the_id_migrates_session_state() {
        let manager = DefaultSessionManager::new();
        let handle = manager.create_session_with_id("old".to_string());
        handle.lock().unwrap().set_attribute("user", "alice".into());

        let new_id = manager.change_session_id("old").expect("new id");

        assert_ne!(new_id, "old");
        assert!(!manager.contains_session("old"));
        let migrated = manager.get_session(&new_id).expect("migrated session");
        assert_eq!(
            migrated
                .lock()
                .unwrap()
                .attribute("user")
                .and_then(|value| value.as_str()),
            Some("alice")
        );
    }

    #[test]
    fn invalidating_a_session_unregisters_and_revokes_it() {
        let manager = DefaultSessionManager::new();
        let handle = manager.create_session_with_id("s1".to_string());

        assert!(manager.invalidate_session("s1"));

        assert!(!manager.contains_session("s1"));
        assert!(!handle.lock().unwrap().is_valid());
    }

    #[test]
    fn accessor_reads_the_session_through_the_manager() {
        let manager = DefaultSessionManager::shared();
        let handle = manager.create_session_with_id("s1".to_string());
        handle.lock().unwrap().set_attribute("user", "alice".into());

        let accessor = handle
            .lock()
            .unwrap()
            .accessor()
            .expect("a shared manager enables accessors");

        let seen: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let sink = Arc::clone(&seen);
        accessor.access(Box::new(move |session| {
            *sink.lock().unwrap() = session
                .attribute("user")
                .and_then(|value| value.as_str())
                .map(str::to_owned);
        }));

        assert_eq!(seen.lock().unwrap().as_deref(), Some("alice"));
    }

    #[test]
    fn a_plain_manager_does_not_offer_an_accessor() {
        let manager = DefaultSessionManager::new();
        let handle = manager.create_session_with_id("s1".to_string());

        assert!(handle.lock().unwrap().accessor().is_none());
    }
}
