//! The default retry context, and the base of every policy-specific context.

use std::{
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU16, Ordering},
    },
};

use next_web_core::any::AnyValue;

use crate::{
    error::retry_error::RetryError,
    retry_context::{AttributeAccessorSupport, RetryContext, SyncAttributeAccessor},
};

/// The default retry context.
///
/// It implements the parts of a retry context that never vary: a parent link, an attempt
/// counter, the last error, the exhausted flag and an attribute store. Policies that need
/// extra state wrap it and delegate through [`crate::impl_retry_context`].
pub struct RetryContextSupport {
    parent: Option<Arc<dyn RetryContext>>,
    count: AtomicU16,
    terminate: AtomicBool,
    last_error: Mutex<Option<RetryError>>,
    attribute_support: AttributeAccessorSupport,
}

impl RetryContextSupport {
    /// Creates a context with the given parent.
    ///
    /// The parent is the context of the enclosing retry block, if the caller is itself
    /// running inside one. It is only used for reporting; every policy keeps its own
    /// attempt counter.
    pub fn with_parent(parent: Option<Arc<dyn RetryContext>>) -> Self {
        Self {
            parent,
            count: AtomicU16::new(0),
            terminate: AtomicBool::new(false),
            last_error: Mutex::new(None),
            attribute_support: AttributeAccessorSupport::default(),
        }
    }

    /// Creates a context without a parent.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a failed attempt.
    ///
    /// This sets the last error and increments the attempt counter, so it must be called
    /// exactly once per failed attempt.
    pub fn register_error(&self, error: &RetryError) {
        self.count.fetch_add(1, Ordering::AcqRel);
        if let Ok(mut last_error) = self.last_error.lock() {
            *last_error = Some(error.clone());
        }
    }

    /// Returns the parent context as a shared handle, if there is one.
    pub fn get_parent_shared(&self) -> Option<Arc<dyn RetryContext>> {
        self.parent.clone()
    }

    /// Replaces the last error without incrementing the attempt counter.
    pub fn set_last_error(&self, error: Option<RetryError>) {
        if let Ok(mut last_error) = self.last_error.lock() {
            *last_error = error;
        }
    }
}

impl Default for RetryContextSupport {
    fn default() -> Self {
        Self::with_parent(None)
    }
}

impl Clone for RetryContextSupport {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            count: AtomicU16::new(self.count.load(Ordering::Acquire)),
            terminate: AtomicBool::new(self.terminate.load(Ordering::Acquire)),
            last_error: Mutex::new(self.get_last_error()),
            attribute_support: self.attribute_support.clone(),
        }
    }
}

impl fmt::Debug for RetryContextSupport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RetryContextSupport")
            .field("count", &self.get_retry_count())
            .field("last_error", &self.get_last_error())
            .field("exhausted", &self.is_exhausted_only())
            .finish()
    }
}

impl fmt::Display for RetryContextSupport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[RetryContext: count={}, lastError={}, exhausted={}]",
            self.get_retry_count(),
            self.get_last_error()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            self.is_exhausted_only()
        )
    }
}

impl SyncAttributeAccessor for RetryContextSupport {
    fn has_attribute(&self, name: &str) -> bool {
        self.attribute_support.has_attribute(name)
    }

    fn set_attribute(&self, name: &str, value: AnyValue) {
        self.attribute_support.set_attribute(name, value)
    }

    fn remove_attribute(&self, name: &str) -> Option<AnyValue> {
        self.attribute_support.remove_attribute(name)
    }

    fn get_attribute(&self, name: &str) -> Option<AnyValue> {
        self.attribute_support.get_attribute(name)
    }
}

impl RetryContext for RetryContextSupport {
    fn set_exhausted_only(&self) {
        self.terminate.store(true, Ordering::Release);
    }

    fn is_exhausted_only(&self) -> bool {
        self.terminate.load(Ordering::Acquire)
    }

    fn get_parent(&self) -> Option<&dyn RetryContext> {
        self.parent.as_deref()
    }

    fn get_parent_shared(&self) -> Option<Arc<dyn RetryContext>> {
        self.parent.clone()
    }

    fn get_retry_count(&self) -> u16 {
        self.count.load(Ordering::Acquire)
    }

    fn get_last_error(&self) -> Option<RetryError> {
        self.last_error
            .lock()
            .ok()
            .and_then(|last_error| last_error.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registering_errors_increments_the_count_and_keeps_the_last_one() {
        let context = RetryContextSupport::new();

        context.register_error(&RetryError::custom("first"));
        context.register_error(&RetryError::custom("second"));

        assert_eq!(context.get_retry_count(), 2);
        assert_eq!(
            context.get_last_error().map(|error| error.message()),
            Some("second".to_owned())
        );
    }

    #[test]
    fn a_clone_does_not_share_the_attempt_counter() {
        let context = RetryContextSupport::new();
        context.register_error(&RetryError::custom("first"));

        let clone = context.clone();
        clone.register_error(&RetryError::custom("second"));

        assert_eq!(context.get_retry_count(), 1);
        assert_eq!(clone.get_retry_count(), 2);
    }

    #[test]
    fn the_exhausted_flag_is_shared_between_tasks() {
        let context = Arc::new(RetryContextSupport::new());
        let writer = context.clone();

        std::thread::spawn(move || writer.set_exhausted_only())
            .join()
            .expect("thread should finish");

        assert!(context.is_exhausted_only());
    }

    #[test]
    fn the_parent_is_reported() {
        let parent: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::new());
        let context = RetryContextSupport::with_parent(Some(parent));

        assert!(context.get_parent().is_some());
    }
}
