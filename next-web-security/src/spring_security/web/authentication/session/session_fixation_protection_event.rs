use std::any::TypeId;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use next_web_context::ApplicationEvent;

use crate::authentication::event::BaseAuthenticationEvent;
use crate::core::Authentication;

/// Indicates a session ID was changed for the purposes of session fixation protection.
///
/// # See Also
///
/// * `SessionFixationProtectionStrategy`
#[derive(Clone)]
pub struct SessionFixationProtectionEvent {
    old_session_id: String,
    new_session_id: String,

    inner: BaseAuthenticationEvent,
}

impl SessionFixationProtectionEvent {
    /// Constructs a new session fixation protection event.
    ///
    /// # Arguments
    ///
    /// * `authentication` - The authentication object.
    /// * `old_session_id` - The old session ID before it was changed. Must not be empty.
    /// * `new_session_id` - The new session ID after it was changed. Must not be empty.
    ///
    /// # Panics
    ///
    /// Panics if `old_session_id` or `new_session_id` is empty.
    pub fn new(
        authentication: Arc<dyn Authentication>,
        old_session_id: &str,
        new_session_id: &str,
    ) -> Self {
        assert!(
            !old_session_id.is_empty(),
            "old_session_id must have length"
        );
        assert!(
            !new_session_id.is_empty(),
            "new_session_id must have length"
        );
        Self {
            old_session_id: old_session_id.to_string(),
            new_session_id: new_session_id.to_string(),

            inner: BaseAuthenticationEvent::new(authentication),
        }
    }

    /// Getter for the session ID before it was changed.
    ///
    /// # Returns
    ///
    /// The old session ID.
    pub fn get_old_session_id(&self) -> &str {
        &self.old_session_id
    }

    /// Getter for the session ID after it was changed.
    ///
    /// # Returns
    ///
    /// The new session ID.
    pub fn get_new_session_id(&self) -> &str {
        &self.new_session_id
    }
}

impl ApplicationEvent for SessionFixationProtectionEvent {
    fn timestamp(&self) -> u64 {
        self.inner.timestamp()
    }

    fn source(&self) -> &dyn std::any::Any {
        self.inner.source()
    }

    fn event_type(&self) -> std::any::TypeId {
        TypeId::of::<Self>()
    }

    fn source_type(&self) -> TypeId {
        self.inner.source_type()
    }
}

// Implement Deref and DerefMut to simulate Java inheritance for AbstractAuthenticationEvent.
impl Deref for SessionFixationProtectionEvent {
    type Target = BaseAuthenticationEvent;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for SessionFixationProtectionEvent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
