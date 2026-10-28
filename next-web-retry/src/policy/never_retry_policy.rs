//! A policy that allows the first attempt and never a retry.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use next_web_core::Named;

use crate::{
    context::retry_context_support::RetryContextSupport, error::retry_error::RetryError,
    impl_retry_context, retry_context::RetryContext, retry_policy::RetryPolicy,
};

/// A policy that allows the first attempt and never a retry.
///
/// Its context remembers whether an attempt already failed, which is why it is also the
/// building block of [`AlwaysRetryPolicy`](super::always_retry_policy::AlwaysRetryPolicy).
#[derive(Clone, Default)]
pub struct NeverRetryPolicy;

impl NeverRetryPolicy {
    /// Creates the policy.
    pub fn new() -> Self {
        Self
    }
}

impl RetryPolicy for NeverRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {
        let any: &dyn std::any::Any = context;
        any.downcast_ref::<NeverRetryContext>()
            .map(|context| !context.is_finished())
            .unwrap_or(true)
    }

    fn open(&self, parent: Option<Arc<dyn RetryContext>>) -> Arc<dyn RetryContext> {
        Arc::new(NeverRetryContext {
            context_support: RetryContextSupport::with_parent(parent),
            finished: AtomicBool::new(false),
        })
    }

    fn close(&self, _context: &dyn RetryContext) {}

    fn register_error(&self, context: &dyn RetryContext, error: &RetryError) {
        let any: &dyn std::any::Any = context;
        if let Some(context) = any.downcast_ref::<NeverRetryContext>() {
            context.finished.store(true, Ordering::Release);
            context.context_support.register_error(error);
        }
    }
}

impl Named for NeverRetryPolicy {
    fn name(&self) -> &str {
        "NeverRetryPolicy"
    }
}

/// The context of a [`NeverRetryPolicy`].
///
/// The finished flag is kept separate from [`RetryContext::is_exhausted_only`] so a
/// subclass can change what `can_retry` answers without changing the meaning of the
/// exhausted flag for the template.
#[derive(Debug)]
struct NeverRetryContext {
    context_support: RetryContextSupport,
    finished: AtomicBool,
}

impl NeverRetryContext {
    fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }
}

impl_retry_context!(NeverRetryContext);

#[cfg(test)]
mod tests {
    use super::NeverRetryPolicy;
    use crate::{error::retry_error::RetryError, retry_policy::RetryPolicy};

    #[test]
    fn the_first_attempt_is_allowed_and_the_retry_is_not() {
        let policy = NeverRetryPolicy::new();
        let context = policy.open(None);

        assert!(policy.can_retry(context.as_ref()));
        policy.register_error(context.as_ref(), &RetryError::custom("failed"));
        assert!(!policy.can_retry(context.as_ref()));
        assert_eq!(context.get_retry_count(), 1);
    }
}
