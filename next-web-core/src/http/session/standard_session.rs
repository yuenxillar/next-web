//! The standard, in-memory [`HttpSession`] implementation.
//!
//! [`StandardSession`] keeps every piece of session state in plain fields, so
//! the getters required by [`HttpSession`] (which borrow, for example
//! [`HttpSession::attribute`]) can return references without locking. Sessions
//! are therefore owned and mutated through `&mut self` during a request; a
//! [`SessionManager`] stores them and hands out shared handles when
//! cross-request access is needed.

use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Weak},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    any::AnyValue,
    http::session::{HttpSession, HttpSessionAccessor, SessionManager},
};

/// Returns the current wall-clock time in milliseconds since the Unix epoch.
///
/// A clock set before the epoch is clamped to the epoch so the result is always
/// representable as a `u64`.
pub(crate) fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// A concrete, in-memory [`HttpSession`].
///
/// The struct tracks the identifier, creation/access timestamps, the maximum
/// inactive interval, the "new" flag, an invalidation flag, and the bound
/// attributes. It is intentionally a plain value type:
///
/// * [`HttpSession::attribute`] can hand out a borrowed [`AnyValue`] without a
///   lock, which a shared/locked map could not do;
/// * the hosting [`SessionManager`] stores the session behind an
///   `Arc<Mutex<..>>` and performs locking on the caller's behalf.
///
/// # Examples
///
/// ```
/// use next_web_core::http::session::{HttpSession, StandardSession};
///
/// let mut session = StandardSession::new("session-1");
/// assert!(session.is_new());
/// assert_eq!(session.id(), "session-1");
///
/// session.set_attribute("user", "alice".into());
/// assert_eq!(session.attribute("user").and_then(|v| v.as_str()), Some("alice"));
///
/// session.invalidate();
/// assert!(session.attribute("user").is_none());
/// ```
pub struct StandardSession {
    /// The unique session identifier. Immutable for the lifetime of the
    /// session; a new id means a new session (see
    /// [`SessionManager::change_session_id`]).
    id: String,
    /// Creation timestamp in milliseconds since the Unix epoch.
    creation_time: u64,
    /// Timestamp of the last recorded access, in milliseconds since the Unix
    /// epoch.
    last_accessed_time: u64,
    /// Idle timeout in seconds. `0` means the session never times out.
    max_inactive_interval: u64,
    /// Whether the client is not yet known to have joined the session.
    is_new: bool,
    /// Whether the session is still usable. Cleared by
    /// [`HttpSession::invalidate`].
    valid: bool,
    /// Attributes bound to the session.
    attributes: HashMap<String, AnyValue>,
    /// Weak handle to the owning manager, used to build an
    /// [`HttpSessionAccessor`]. Kept weak so a manager is never kept alive by
    /// the sessions it stores.
    manager: Option<Weak<dyn SessionManager>>,
}

impl StandardSession {
    /// Creates a session with the given identifier.
    ///
    /// The session starts as new and valid, with no attributes and no idle
    /// timeout. A [`SessionManager`] normally sets the timeout through
    /// [`HttpSession::set_max_inactive_interval`] right after creation.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        let now = current_time_millis();
        Self {
            id: id.into(),
            creation_time: now,
            last_accessed_time: now,
            max_inactive_interval: 0,
            is_new: true,
            valid: true,
            attributes: HashMap::new(),
            manager: None,
        }
    }

    /// Attaches the owning manager so [`HttpSession::accessor`] can resolve the
    /// session outside of a request.
    ///
    /// The reference is weak, so this does not create a reference cycle.
    pub(crate) fn attach_manager(&mut self, manager: Weak<dyn SessionManager>) {
        self.manager = Some(manager);
    }

    /// Records an access to the session by refreshing the last-accessed time.
    ///
    /// The container calls this at the start (or end) of every request that is
    /// associated with the session.
    pub fn touch(&mut self) {
        self.last_accessed_time = current_time_millis();
    }

    /// Returns `true` while the session has not been invalidated.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Overrides the "new" flag.
    ///
    /// A container clears the flag once the client demonstrates that it has
    /// received the session identifier (for example by echoing the session
    /// cookie), which is how [`HttpSession::is_new`] stops reporting `true`.
    pub fn set_new(&mut self, is_new: bool) {
        self.is_new = is_new;
    }

    /// Overrides the last-accessed timestamp (milliseconds since the epoch).
    pub fn set_last_accessed_time(&mut self, milliseconds: u64) {
        self.last_accessed_time = milliseconds;
    }

    /// Returns `true` when the session should be discarded at `now`
    /// (milliseconds since the epoch).
    ///
    /// A session is expired when it has been invalidated, or when it has a
    /// positive idle timeout and has not been accessed within it.
    #[must_use]
    pub fn is_expired(&self, now: u64) -> bool {
        if !self.valid {
            return true;
        }
        if self.max_inactive_interval == 0 {
            return false;
        }
        let idle = now.saturating_sub(self.last_accessed_time);
        idle > self.max_inactive_interval.saturating_mul(1000)
    }

    /// Copies the mutable state of `other` into this session.
    ///
    /// The identifier and the attached manager are left untouched; timeouts,
    /// flags, timestamps, and attributes are replaced. This is what
    /// [`SessionManager::change_session_id`] uses to migrate a session onto a
    /// fresh identifier.
    pub fn copy_state_from(&mut self, other: &StandardSession) {
        self.creation_time = other.creation_time;
        self.last_accessed_time = other.last_accessed_time;
        self.max_inactive_interval = other.max_inactive_interval;
        self.is_new = other.is_new;
        self.valid = other.valid;
        self.attributes = other.attributes.clone();
    }

    /// Returns the attributes currently bound to the session.
    #[must_use]
    pub fn attributes(&self) -> &HashMap<String, AnyValue> {
        &self.attributes
    }
}

impl fmt::Debug for StandardSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StandardSession")
            .field("id", &self.id)
            .field("creation_time", &self.creation_time)
            .field("last_accessed_time", &self.last_accessed_time)
            .field("max_inactive_interval", &self.max_inactive_interval)
            .field("is_new", &self.is_new)
            .field("valid", &self.valid)
            .field("attributes", &self.attributes.len())
            .finish()
    }
}

impl HttpSession for StandardSession {
    fn creation_time(&self) -> u64 {
        self.creation_time
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn last_accessed_time(&self) -> u64 {
        self.last_accessed_time
    }

    fn set_max_inactive_interval(&mut self, interval: u64) {
        self.max_inactive_interval = interval;
    }

    fn max_inactive_interval(&self) -> u64 {
        self.max_inactive_interval
    }

    fn attribute(&self, name: &str) -> Option<&AnyValue> {
        self.attributes.get(name)
    }

    fn attribute_names(&self) -> Vec<&str> {
        self.attributes.keys().map(String::as_str).collect()
    }

    fn set_attribute(&mut self, name: &str, value: AnyValue) {
        self.attributes.insert(name.to_owned(), value);
    }

    fn remove_attribute(&mut self, name: &str) {
        self.attributes.remove(name);
    }

    fn invalidate(&mut self) {
        self.attributes.clear();
        self.valid = false;
        self.is_new = false;
    }

    fn is_new(&self) -> bool {
        self.is_new
    }

    fn accessor(&self) -> Option<Box<dyn HttpSessionAccessor>> {
        let manager = self.manager.as_ref()?.upgrade()?;
        Some(Box::new(StandardSessionAccessor::new(
            manager,
            self.id.clone(),
        )))
    }
}

/// An [`HttpSessionAccessor`] that resolves a session by id through its
/// [`SessionManager`].
///
/// The accessor is produced by [`StandardSession::accessor`] and remains valid
/// only while the identifier resolves to a live session: after a session id
/// change or invalidation, [`HttpSessionAccessor::access`] simply does not
/// invoke its consumer.
pub struct StandardSessionAccessor {
    /// The manager used to look the session up.
    manager: Arc<dyn SessionManager>,
    /// The identifier the accessor is bound to.
    session_id: String,
}

impl StandardSessionAccessor {
    /// Creates an accessor for `session_id` backed by `manager`.
    #[must_use]
    pub fn new(manager: Arc<dyn SessionManager>, session_id: impl Into<String>) -> Self {
        Self {
            manager,
            session_id: session_id.into(),
        }
    }

    /// Returns the identifier this accessor is bound to.
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

impl fmt::Debug for StandardSessionAccessor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StandardSessionAccessor")
            .field("session_id", &self.session_id)
            .finish_non_exhaustive()
    }
}

impl HttpSessionAccessor for StandardSessionAccessor {
    fn access(&self, session_consumer: Box<dyn FnOnce(&dyn HttpSession)>) {
        let mut consumer = Some(session_consumer);
        self.manager.with_session(&self.session_id, &mut |session| {
            if let Some(consumer) = consumer.take() {
                consumer(session);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::StandardSession;
    use crate::http::session::HttpSession;

    #[test]
    fn attributes_can_be_set_read_and_removed() {
        let mut session = StandardSession::new("s1");

        session.set_attribute("user", "alice".into());
        assert_eq!(
            session.attribute("user").and_then(|value| value.as_str()),
            Some("alice")
        );
        assert_eq!(session.attribute_names(), vec!["user"]);

        session.remove_attribute("user");
        assert!(session.attribute("user").is_none());
        assert!(session.attribute_names().is_empty());
    }

    #[test]
    fn invalidate_clears_attributes_and_marks_the_session_invalid() {
        let mut session = StandardSession::new("s1");
        session.set_attribute("user", "alice".into());
        session.set_new(false);

        session.invalidate();

        assert!(!session.is_valid());
        assert!(session.is_expired(super::current_time_millis()));
        assert!(!session.is_new());
        assert!(session.attributes().is_empty());
    }

    #[test]
    fn idle_timeout_expires_only_after_the_interval() {
        let mut session = StandardSession::new("s1");
        session.set_max_inactive_interval(30);
        session.set_last_accessed_time(1_000_000);

        // 30 seconds of inactivity is not yet expired; anything beyond it is.
        assert!(!session.is_expired(1_000_000 + 30 * 1000));
        assert!(session.is_expired(1_000_000 + 30 * 1000 + 1));
    }

    #[test]
    fn a_zero_interval_never_expires() {
        let session = StandardSession::new("s1");

        assert!(!session.is_expired(u64::MAX));
    }
}
