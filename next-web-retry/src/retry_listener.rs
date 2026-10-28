//! Listeners observe the lifecycle of a retry block.

use std::any::Any;

use next_web_core::any::AnyError;

use crate::retry_context::RetryContext;

/// Observes the lifecycle of a retry block.
///
/// Listeners run around the whole block, and on each failed or successful attempt:
///
/// ```text
/// open -> [ attempt -> (on_error | on_success) ]* -> close
/// ```
///
/// Every hook has an empty, permissive default, so a listener only implements what it
/// needs. Returning `false` from [`RetryListener::open`] vetoes the retry and terminates it
/// with a [`crate::error::retry_error::RetryError::TerminatedRetryError`].
pub trait RetryListener
where
    Self: Send + Sync,
{
    /// Called before the first attempt. Returning `false` terminates the retry.
    fn open(&self, _context: &dyn RetryContext) -> bool {
        true
    }

    /// Called after the last attempt, whether it succeeded or not.
    ///
    /// `error` is the last error of the block, or `None` when the block succeeded.
    fn close(&self, _context: &dyn RetryContext, _error: Option<&dyn AnyError>) {}

    /// Called after a successful attempt, with the produced value.
    fn on_success(&self, _context: &dyn RetryContext, _result: &dyn Any) {}

    /// Called after every failed attempt.
    fn on_error(&self, _context: &dyn RetryContext, _error: &dyn AnyError) {}
}

/// A listener that does nothing.
///
/// Registered by default so a template always has a stable listener list.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultRetryListener {}

impl RetryListener for DefaultRetryListener {}

/// Fans one lifecycle event out to several listeners.
///
/// [`RetryListener::open`] stops at the first listener that vetoes the retry; the other
/// hooks call every listener in order.
pub struct CompositeRetryListener {
    listeners: Vec<std::sync::Arc<dyn RetryListener>>,
}

impl CompositeRetryListener {
    /// Creates a composite listener from the given listeners.
    pub fn new(listeners: Vec<std::sync::Arc<dyn RetryListener>>) -> Self {
        Self { listeners }
    }

    /// Returns the number of listeners in this composite.
    pub fn len(&self) -> usize {
        self.listeners.len()
    }

    /// Returns whether this composite has no listeners.
    pub fn is_empty(&self) -> bool {
        self.listeners.is_empty()
    }
}

impl RetryListener for CompositeRetryListener {
    fn open(&self, context: &dyn RetryContext) -> bool {
        self.listeners.iter().all(|listener| listener.open(context))
    }

    fn close(&self, context: &dyn RetryContext, error: Option<&dyn AnyError>) {
        for listener in self.listeners.iter().rev() {
            listener.close(context, error);
        }
    }

    fn on_success(&self, context: &dyn RetryContext, result: &dyn Any) {
        for listener in self.listeners.iter().rev() {
            listener.on_success(context, result);
        }
    }

    fn on_error(&self, context: &dyn RetryContext, error: &dyn AnyError) {
        for listener in self.listeners.iter().rev() {
            listener.on_error(context, error);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };

    use next_web_core::any::AnyError;

    use super::{CompositeRetryListener, RetryListener};
    use crate::{
        context::retry_context_support::RetryContextSupport, error::retry_error::RetryError,
        retry_context::RetryContext,
    };

    #[derive(Default)]
    struct VetoListener {
        opened: AtomicBool,
    }

    impl RetryListener for VetoListener {
        fn open(&self, _context: &dyn RetryContext) -> bool {
            self.opened.store(true, Ordering::Release);
            false
        }
    }

    struct RecordingListener {
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    impl RetryListener for RecordingListener {
        fn close(&self, _context: &dyn RetryContext, _error: Option<&dyn AnyError>) {
            if let Ok(mut events) = self.events.lock() {
                events.push("close");
            }
        }

        fn on_error(&self, _context: &dyn RetryContext, _error: &dyn AnyError) {
            if let Ok(mut events) = self.events.lock() {
                events.push("error");
            }
        }
    }

    #[test]
    fn a_vetoing_listener_stops_the_composite_from_opening() {
        let composite = CompositeRetryListener::new(vec![Arc::new(VetoListener::default())]);
        let context = RetryContextSupport::new();

        assert!(!composite.open(&context));
    }

    #[test]
    fn reverse_hooks_run_for_every_listener() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let composite = CompositeRetryListener::new(vec![
            Arc::new(RecordingListener {
                events: events.clone(),
            }),
            Arc::new(RecordingListener {
                events: events.clone(),
            }),
        ]);
        let context = RetryContextSupport::new();
        let error = RetryError::custom("failed");

        composite.on_error(&context, &error);
        composite.close(&context, Some(&error));

        assert_eq!(
            events
                .lock()
                .map(|events| events.clone())
                .unwrap_or_default(),
            ["error", "error", "close", "close"]
        );
        assert!(!composite.is_empty());
        assert_eq!(composite.len(), 2);
    }
}
