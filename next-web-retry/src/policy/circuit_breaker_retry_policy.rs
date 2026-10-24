//! A policy that stops retrying for a while after a run of failures.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use next_web_core::Named;
use tracing::trace;

use crate::{
    context::retry_context_support::RetryContextSupport,
    error::retry_error::RetryError,
    retry_context::{RetryContext, SyncAttributeAccessor, retry_context_constants},
    retry_policy::RetryPolicy,
};

/// The context attribute that reports whether the circuit is open.
pub const CIRCUIT_OPEN: &str = "circuit.open";

/// The context attribute that counts how many attempts were short-circuited.
pub const CIRCUIT_SHORT_COUNT: &str = "circuit.shortCount";

/// The default time a circuit stays open, in milliseconds.
pub const DEFAULT_RESET_TIMEOUT: u64 = 20_000;

/// The default window during which a failure trips the circuit, in milliseconds.
pub const DEFAULT_OPEN_TIMEOUT: u64 = 5_000;

/// A policy that stops retrying for a while after a run of failures.
///
/// The delegate policy decides when attempts are exhausted. When it does, the circuit opens
/// for [`CircuitBreakerRetryPolicy::set_reset_timeout`] milliseconds: every call in that
/// window fails immediately instead of reaching the callback. After the window the circuit
/// closes and a fresh delegate context starts.
///
/// The context carries [`retry_context_constants::GLOBAL_STATE`], so the template keeps it
/// in the dedicated circuit-breaker cache instead of the stateful one and never closes it.
pub struct CircuitBreakerRetryPolicy {
    delegate: Arc<dyn RetryPolicy>,
    reset_timeout: u64,
    open_timeout: u64,
    reset_timeout_supplier: Option<Arc<dyn Fn() -> u64 + Send + Sync>>,
    open_timeout_supplier: Option<Arc<dyn Fn() -> u64 + Send + Sync>>,
}

impl CircuitBreakerRetryPolicy {
    /// Creates a policy around a delegate.
    pub fn new(delegate: Arc<dyn RetryPolicy>) -> Self {
        Self {
            delegate,
            reset_timeout: DEFAULT_RESET_TIMEOUT,
            open_timeout: DEFAULT_OPEN_TIMEOUT,
            reset_timeout_supplier: None,
            open_timeout_supplier: None,
        }
    }

    /// Creates a policy around a [`SimpleRetryPolicy`](super::simple_retry_policy::SimpleRetryPolicy).
    pub fn with_simple_delegate() -> Self {
        Self::new(Arc::new(
            super::simple_retry_policy::SimpleRetryPolicy::default(),
        ))
    }

    /// Sets how long the circuit stays open.
    pub fn set_reset_timeout(&mut self, timeout_millis: u64) {
        self.reset_timeout = timeout_millis;
    }

    /// Installs a dynamic source for the reset timeout.
    pub fn reset_timeout_supplier<F>(&mut self, supplier: F)
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        self.reset_timeout_supplier = Some(Arc::new(supplier));
    }

    /// Sets the window during which a failure trips the circuit.
    pub fn set_open_timeout(&mut self, timeout_millis: u64) {
        self.open_timeout = timeout_millis;
    }

    /// Installs a dynamic source for the open timeout.
    pub fn open_timeout_supplier<F>(&mut self, supplier: F)
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        self.open_timeout_supplier = Some(Arc::new(supplier));
    }

    /// Returns the delegate policy.
    pub fn delegate(&self) -> &Arc<dyn RetryPolicy> {
        &self.delegate
    }

    fn current_reset_timeout(&self) -> u64 {
        self.reset_timeout_supplier
            .as_ref()
            .map(|supplier| supplier())
            .unwrap_or(self.reset_timeout)
    }

    fn current_open_timeout(&self) -> u64 {
        self.open_timeout_supplier
            .as_ref()
            .map(|supplier| supplier())
            .unwrap_or(self.open_timeout)
    }
}

impl Clone for CircuitBreakerRetryPolicy {
    fn clone(&self) -> Self {
        Self {
            delegate: self.delegate.clone(),
            reset_timeout: self.reset_timeout,
            open_timeout: self.open_timeout,
            reset_timeout_supplier: self.reset_timeout_supplier.clone(),
            open_timeout_supplier: self.open_timeout_supplier.clone(),
        }
    }
}

impl RetryPolicy for CircuitBreakerRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {
        let any: &dyn std::any::Any = context;
        let Some(context) = any.downcast_ref::<CircuitBreakerRetryContext>() else {
            return false;
        };

        if context.is_open() {
            context.increment_short_circuit_count();
            false
        } else {
            context.reset_short_circuit_count();
            self.delegate.can_retry(context.delegate_context().as_ref())
        }
    }

    fn open(&self, parent: Option<Arc<dyn RetryContext>>) -> Arc<dyn RetryContext> {
        Arc::new(CircuitBreakerRetryContext::new(
            parent,
            self.delegate.clone(),
            Duration::from_millis(self.current_reset_timeout()),
            Duration::from_millis(self.current_open_timeout()),
        ))
    }

    fn close(&self, context: &dyn RetryContext) {
        let any: &dyn std::any::Any = context;
        if let Some(context) = any.downcast_ref::<CircuitBreakerRetryContext>() {
            self.delegate.close(context.delegate_context().as_ref());
        }
    }

    fn register_error(&self, context: &dyn RetryContext, error: &RetryError) {
        let any: &dyn std::any::Any = context;
        if let Some(context) = any.downcast_ref::<CircuitBreakerRetryContext>() {
            context.context_support.register_error(error);
            self.delegate
                .register_error(context.delegate_context().as_ref(), error);
        }
    }
}

impl Named for CircuitBreakerRetryPolicy {
    fn name(&self) -> &str {
        "CircuitBreakerRetryPolicy"
    }
}

/// The state of a circuit: the delegate context and when it was started.
struct CircuitState {
    context: Arc<dyn RetryContext>,
    start: Instant,
}

/// The context of a [`CircuitBreakerRetryPolicy`].
///
/// It wraps the delegate context and reports the delegate attempt count. The circuit state
/// (open or closed, and how many calls were short-circuited) is stored in attributes so
/// listeners and metrics can observe it.
struct CircuitBreakerRetryContext {
    context_support: RetryContextSupport,
    policy: Arc<dyn RetryPolicy>,
    state: Mutex<CircuitState>,
    timeout: Duration,
    open_window: Duration,
}

impl CircuitBreakerRetryContext {
    fn new(
        parent: Option<Arc<dyn RetryContext>>,
        policy: Arc<dyn RetryPolicy>,
        timeout: Duration,
        open_window: Duration,
    ) -> Self {
        let context = policy.open(parent.clone());
        let context = Self {
            context_support: RetryContextSupport::with_parent(parent),
            policy,
            state: Mutex::new(CircuitState {
                context,
                start: Instant::now(),
            }),
            timeout,
            open_window,
        };
        context.reset_short_circuit_count();
        context.set_attribute(retry_context_constants::GLOBAL_STATE, true.into());
        context
    }

    fn delegate_context(&self) -> Arc<dyn RetryContext> {
        match self.state.lock() {
            Ok(state) => state.context.clone(),
            Err(poisoned) => poisoned.into_inner().context.clone(),
        }
    }

    fn store_state(&self, context: Arc<dyn RetryContext>, start: Instant) {
        match self.state.lock() {
            Ok(mut state) => {
                state.context = context;
                state.start = start;
            }
            Err(poisoned) => {
                let mut state = poisoned.into_inner();
                state.context = context;
                state.start = start;
            }
        }
    }

    fn start_instant(&self) -> Instant {
        match self.state.lock() {
            Ok(state) => state.start,
            Err(poisoned) => poisoned.into_inner().start,
        }
    }

    fn create_delegate_context(&self) -> Arc<dyn RetryContext> {
        self.reset_short_circuit_count();
        self.policy.open(self.context_support.get_parent_shared())
    }

    fn reset_short_circuit_count(&self) {
        self.set_attribute(CIRCUIT_SHORT_COUNT, 0u64.into());
    }

    fn increment_short_circuit_count(&self) {
        let current = self
            .get_attribute(CIRCUIT_SHORT_COUNT)
            .and_then(|value| value.as_u64())
            .unwrap_or_default();
        self.set_attribute(CIRCUIT_SHORT_COUNT, current.saturating_add(1).into());
    }

    /// Returns whether the circuit is open, updating its state on the way.
    fn is_open(&self) -> bool {
        let mut context = self.delegate_context();
        let mut start = self.start_instant();
        let time = start.elapsed();
        let mut retryable = self.policy.can_retry(context.as_ref());

        if !retryable {
            if time > self.timeout {
                trace!("Circuit breaker closing, the reset timeout elapsed");
                context = self.create_delegate_context();
                start = Instant::now();
                retryable = self.policy.can_retry(context.as_ref());
            } else if time < self.open_window {
                if !self.has_attribute(CIRCUIT_OPEN) || !self.flag(CIRCUIT_OPEN) {
                    trace!("Circuit breaker opening");
                    self.set_attribute(CIRCUIT_OPEN, true.into());
                    start = Instant::now();
                }
                self.store_state(context, start);
                return true;
            }
        } else if time > self.open_window {
            trace!("Circuit breaker resetting its delegate context");
            start = Instant::now();
            context = self.create_delegate_context();
        }

        self.store_state(context, start);
        self.set_attribute(CIRCUIT_OPEN, (!retryable).into());
        !retryable
    }
}

impl crate::retry_context::SyncAttributeAccessor for CircuitBreakerRetryContext {
    fn has_attribute(&self, name: &str) -> bool {
        self.context_support.has_attribute(name)
    }

    fn set_attribute(&self, name: &str, value: next_web_core::any::AnyValue) {
        self.context_support.set_attribute(name, value);
    }

    fn remove_attribute(&self, name: &str) -> Option<next_web_core::any::AnyValue> {
        self.context_support.remove_attribute(name)
    }

    fn get_attribute(&self, name: &str) -> Option<next_web_core::any::AnyValue> {
        self.context_support.get_attribute(name)
    }
}

impl RetryContext for CircuitBreakerRetryContext {
    fn set_exhausted_only(&self) {
        self.context_support.set_exhausted_only();
    }

    fn is_exhausted_only(&self) -> bool {
        self.context_support.is_exhausted_only()
    }

    fn get_parent(&self) -> Option<&dyn RetryContext> {
        self.context_support.get_parent()
    }

    fn get_parent_shared(&self) -> Option<Arc<dyn RetryContext>> {
        self.context_support.get_parent_shared()
    }

    /// Reports the delegate attempt count: the circuit itself does not count attempts.
    fn get_retry_count(&self) -> u16 {
        self.delegate_context().get_retry_count()
    }

    fn get_last_error(&self) -> Option<RetryError> {
        self.context_support.get_last_error()
    }
}

impl std::fmt::Debug for CircuitBreakerRetryContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CircuitBreakerRetryContext")
            .field("retry_count", &self.get_retry_count())
            .field("open", &self.flag(CIRCUIT_OPEN))
            .field("short_count", &self.get_attribute(CIRCUIT_SHORT_COUNT))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::{CIRCUIT_OPEN, CIRCUIT_SHORT_COUNT, CircuitBreakerRetryPolicy};
    use crate::{
        error::retry_error::RetryError, policy::max_attempts_retry_policy::MaxAttemptsRetryPolicy,
        retry_policy::RetryPolicy,
    };

    #[test]
    fn the_circuit_opens_after_the_delegate_gives_up() {
        let mut policy = CircuitBreakerRetryPolicy::new(Arc::new(MaxAttemptsRetryPolicy::new(1)));
        policy.set_open_timeout(10_000);
        policy.set_reset_timeout(60_000);
        let context = policy.open(None);

        assert!(policy.can_retry(context.as_ref()));
        policy.register_error(context.as_ref(), &RetryError::custom("boom"));

        assert!(!policy.can_retry(context.as_ref()));
        assert!(context.flag(CIRCUIT_OPEN));
        assert_eq!(
            context
                .get_attribute(CIRCUIT_SHORT_COUNT)
                .and_then(|value| value.as_u64()),
            Some(1)
        );

        // Every call while the circuit is open is short-circuited and counted.
        assert!(!policy.can_retry(context.as_ref()));
        assert_eq!(
            context
                .get_attribute(CIRCUIT_SHORT_COUNT)
                .and_then(|value| value.as_u64()),
            Some(2)
        );
    }

    #[test]
    fn the_circuit_closes_again_after_the_reset_timeout() {
        let policy = CircuitBreakerRetryPolicy::new(Arc::new(MaxAttemptsRetryPolicy::new(1)));
        let context = policy.open(None);
        policy.register_error(context.as_ref(), &RetryError::custom("boom"));
        assert!(!policy.can_retry(context.as_ref()));

        std::thread::sleep(Duration::from_millis(10));
        let mut policy = CircuitBreakerRetryPolicy::new(Arc::new(MaxAttemptsRetryPolicy::new(1)));
        policy.set_reset_timeout(1);
        policy.set_open_timeout(1);
        let context = policy.open(None);
        policy.register_error(context.as_ref(), &RetryError::custom("boom"));
        std::thread::sleep(Duration::from_millis(5));

        assert!(policy.can_retry(context.as_ref()));
    }

    #[test]
    fn the_global_state_attribute_marks_the_context() {
        let policy = CircuitBreakerRetryPolicy::with_simple_delegate();
        let context = policy.open(None);

        assert!(context.flag(crate::retry_context::retry_context_constants::GLOBAL_STATE));
        assert_eq!(context.get_retry_count(), 0);
    }
}
