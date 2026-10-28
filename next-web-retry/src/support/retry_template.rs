//! The template that executes an operation with retry behaviour.

use std::{
    any::Any,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use next_web_core::{
    any::{AnyError, AnyValue},
    async_trait,
};
use tracing::{debug, error, trace};

use crate::{
    backoff::{
        back_off_context::BackOffContext, back_off_policy::BackOffPolicy,
        no_back_off_policy::NoBackOffPolicy,
    },
    error::retry_error::RetryError,
    policy::{
        map_retry_context_cache::MapRetryContextCache, retry_context_cache::RetryContextCache,
        simple_retry_policy::SimpleRetryPolicy,
    },
    recovery_callback::RecoveryCallback,
    retry_callback::RetryCallback,
    retry_context::{RetryContext, retry_context_constants},
    retry_listener::RetryListener,
    retry_operations::RetryOperations,
    retry_policy::RetryPolicy,
    retry_state::RetryState,
    support::{
        retry_synchronization_manager::RetrySynchronizationManager,
        retry_template_builder::RetryTemplateBuilder,
    },
};

/// The retry context attribute that stores the back-off context of a block.
const BACK_OFF_CONTEXT: &str = "backOffContext";

/// Executes an operation with configurable retry behaviour.
///
/// The template runs the callback until it succeeds or the [`RetryPolicy`] refuses another
/// attempt, waiting between attempts according to the [`BackOffPolicy`]. By default an
/// operation is retried for any error up to three attempts with no wait; a
/// [`RetryTemplateBuilder`] configures both policies and the listeners.
///
/// A template is cheap to clone and safe to share: clones point at the same policies, so a
/// single template can serve a whole service, and reconfiguring it does not disturb a retry
/// that is already running.
#[derive(Clone)]
pub struct RetryTemplate {
    back_off_policy: Arc<dyn BackOffPolicy>,
    retry_policy: Arc<dyn RetryPolicy>,
    listeners: Vec<Arc<dyn RetryListener>>,
    retry_context_cache: CompositeRetryContextCache,
    throw_last_exception_on_exhausted: bool,
}

impl RetryTemplate {
    /// Creates a builder with the default behaviour.
    ///
    /// See [`RetryTemplateBuilder`] for the defaults and for the configuration methods.
    pub fn builder() -> RetryTemplateBuilder {
        RetryTemplateBuilder::default()
    }

    /// Creates a template with the default behaviour.
    pub fn default_instance() -> Self {
        RetryTemplateBuilder::default().build()
    }

    /// Returns the retry policy of this template.
    pub fn retry_policy(&self) -> &Arc<dyn RetryPolicy> {
        &self.retry_policy
    }

    /// Returns the back-off policy of this template.
    pub fn back_off_policy(&self) -> &Arc<dyn BackOffPolicy> {
        &self.back_off_policy
    }

    /// Returns whether the last error is rethrown when a stateful retry is exhausted.
    pub fn throw_last_exception_on_exhausted(&self) -> bool {
        self.throw_last_exception_on_exhausted
    }

    /// Sets whether a stateful retry rethrows the last error instead of reporting
    /// [`RetryError::ExhaustedRetryError`].
    ///
    /// This only applies when a [`RetryState`] is supplied.
    pub fn set_throw_last_exception_on_exhausted(
        &mut self,
        throw_last_exception_on_exhausted: bool,
    ) {
        self.throw_last_exception_on_exhausted = throw_last_exception_on_exhausted;
    }

    /// Replaces the cache used for stateful retries.
    pub fn set_retry_context_cache<T>(&mut self, retry_context_cache: T)
    where
        T: RetryContextCache + 'static,
    {
        self.retry_context_cache = CompositeRetryContextCache {
            stateful: Arc::new(retry_context_cache),
            circuit_breaker: self.retry_context_cache.circuit_breaker.clone(),
        };
    }

    /// Replaces the cache used for circuit-breaker retries.
    pub fn set_circuit_breaker_retry_context_cache<T>(&mut self, circuit_breaker_cache: T)
    where
        T: RetryContextCache + 'static,
    {
        self.retry_context_cache = CompositeRetryContextCache {
            stateful: self.retry_context_cache.stateful.clone(),
            circuit_breaker: Arc::new(circuit_breaker_cache),
        };
    }

    /// Replaces the listeners.
    pub fn set_listeners(&mut self, listeners: Vec<Arc<dyn RetryListener>>) {
        self.listeners = listeners;
    }

    /// Replaces the back-off policy.
    pub fn set_back_off_policy<T>(&mut self, back_off_policy: T)
    where
        T: BackOffPolicy + 'static,
    {
        self.back_off_policy = Arc::new(back_off_policy);
    }

    /// Replaces the back-off policy with a shared one.
    pub fn set_back_off_policy_arc(&mut self, back_off_policy: Arc<dyn BackOffPolicy>) {
        self.back_off_policy = back_off_policy;
    }

    /// Replaces the retry policy.
    pub fn set_retry_policy<T>(&mut self, retry_policy: T)
    where
        T: RetryPolicy + 'static,
    {
        self.retry_policy = Arc::new(retry_policy);
    }

    /// Replaces the retry policy with a shared one.
    pub fn set_retry_policy_arc(&mut self, retry_policy: Arc<dyn RetryPolicy>) {
        self.retry_policy = retry_policy;
    }

    /// Appends a listener at the end of the list.
    pub fn register_listener<T>(&mut self, listener: T)
    where
        T: RetryListener + 'static,
    {
        self.listeners.push(Arc::new(listener));
    }

    /// Inserts a listener at `index`.
    pub fn register_listener_with_index<T>(&mut self, listener: T, index: usize)
    where
        T: RetryListener + 'static,
    {
        let index = index.min(self.listeners.len());
        self.listeners.insert(index, Arc::new(listener));
    }

    /// Returns whether at least one listener is registered.
    pub fn has_listeners(&self) -> bool {
        !self.listeners.is_empty()
    }

    /// Implements the retry loop.
    ///
    /// The structure mirrors the original template step by step so the observable behaviour
    /// is the same: open the policies, let the listeners inspect the context, then attempt
    /// until the policy refuses, waiting in between, and finally recover or rethrow.
    async fn do_execute<T, C>(
        &self,
        retry_callback: C,
        recovery_callback: Option<&dyn RecoveryCallback<T>>,
        state: Option<&dyn RetryState>,
    ) -> Result<T, RetryError>
    where
        T: Send + 'static,
        C: RetryCallback<T>,
    {
        let retry_policy = self.retry_policy.clone();
        let back_off_policy = self.back_off_policy.clone();

        let context = self.open(retry_policy.as_ref(), state)?;
        trace!(count = context.get_retry_count(), "Retry context opened");

        RetrySynchronizationManager::scope(context.clone(), async {
            // The guard is the Rust equivalent of a `finally` block: it runs when this block
            // returns normally, when it returns early, when a callback panics, and when the
            // future is dropped because the caller was cancelled. Cleaning up in `Drop` is
            // what makes those last two cases safe.
            let guard = ExecutionGuard::new(self, retry_policy.as_ref(), context.clone(), state);

            // Publish the metadata before the listeners run, so an `open` hook already sees
            // the reporting name and the attempt budget it is observing.
            if let Some(label) = retry_callback.label() {
                context.set_attribute(retry_context_constants::NAME, label.into());
            }

            if !context.has_attribute(retry_context_constants::MAX_ATTEMPTS) {
                context.set_attribute(
                    retry_context_constants::MAX_ATTEMPTS,
                    u64::from(retry_policy.get_max_attempts()).into(),
                );
            }

            let running = self.do_open_interceptors(context.as_ref());
            if !running {
                // A vetoed block is finished, not aborted: it ends through a normal exit, and the
                // caller sees a terminated-retry error.
                context.set_attribute(retry_context_constants::TERMINATED, true.into());
                guard.mark_completed();
                return Err(RetryError::terminated(
                    "Retry terminated abnormally by interceptor before first attempt",
                ));
            }

            let mut back_off_context = context
                .get_attribute(BACK_OFF_CONTEXT)
                .and_then(|value| value.as_ref_value::<Arc<dyn BackOffContext>>().cloned());

            if back_off_context.is_none() {
                back_off_context = back_off_policy.start(context.as_ref()).await;
                if let Some(started) = back_off_context.as_ref() {
                    context.set_attribute(
                        BACK_OFF_CONTEXT,
                        AnyValue::BoxedValue(Box::new(started.clone())),
                    );
                }
            }

            let label_message = retry_callback
                .label()
                .map(|label| format!("; for: '{label}'"))
                .unwrap_or_default();

            while self.can_retry(retry_policy.as_ref(), context.as_ref())
                && !context.is_exhausted_only()
            {
                debug!(
                    count = context.get_retry_count(),
                    label = %label_message,
                    "Retry attempt"
                );

                // Reset the last error so a successful attempt leaves the close interceptors
                // with nothing to report.
                guard.clear_error();

                match retry_callback.do_with_retry(context.as_ref()).await {
                    Ok(result) => {
                        self.do_on_success_interceptors(context.as_ref(), &result);
                        guard.mark_completed();
                        return Ok(result);
                    }
                    Err(error) => {
                        guard.record_error(&error);

                        let registered = self.register_throwable(
                            retry_policy.as_ref(),
                            state,
                            context.clone(),
                            &error,
                        );
                        self.do_on_error_interceptors(context.as_ref(), &error);
                        if let Err(register_error) = registered {
                            context.set_attribute(retry_context_constants::TERMINATED, true.into());
                            guard.mark_completed();
                            return Err(RetryError::terminated_with_cause(
                                "Could not register throwable",
                                Box::new(register_error),
                            ));
                        }

                        if self.can_retry(retry_policy.as_ref(), context.as_ref())
                            && !context.is_exhausted_only()
                            && let Err(back_off_error) =
                                back_off_policy.backoff(back_off_context.as_deref()).await
                        {
                            // An interrupted wait is a finished block: the caller sees the error,
                            // and it is not a cancellation of the retry itself.
                            guard.mark_completed();
                            return Err(back_off_error);
                        }

                        if self.should_rethrow(retry_policy.as_ref(), context.as_ref(), state) {
                            debug!(
                                count = context.get_retry_count(),
                                label = %label_message,
                                "Rethrowing the failure for a stateful retry"
                            );
                            guard.mark_completed();
                            return Err(error);
                        }
                    }
                }

                // A stateful attempt that is allowed to retry may already have rethrown the
                // failure; reaching this point means something else decided to continue,
                // such as a circuit breaker or a rollback classifier.
                if state.is_some() && context.has_attribute(retry_context_constants::GLOBAL_STATE) {
                    break;
                }
            }

            if state.is_none() {
                debug!(
                    count = context.get_retry_count(),
                    label = %label_message,
                    "Retry failed the last attempt"
                );
            }

            guard.mark_exhausted();
            let outcome = self
                .handle_retry_exhausted(recovery_callback, context.as_ref(), state)
                .await;
            // Marked after the recovery path returns, so a panic inside the recoverer is still
            // reported as an aborted block.
            guard.mark_completed();
            outcome
        })
        .await
    }

    /// Returns whether the policy allows another attempt.
    fn can_retry(&self, retry_policy: &dyn RetryPolicy, context: &dyn RetryContext) -> bool {
        retry_policy.can_retry(context)
    }

    /// Closes the policies and updates the cache.
    fn close(
        &self,
        retry_policy: &dyn RetryPolicy,
        context: &dyn RetryContext,
        state: Option<&dyn RetryState>,
        succeeded: bool,
    ) {
        if let Some(state) = state {
            if succeeded {
                self.retry_context_cache.remove(
                    state.get_key().map(String::as_str).unwrap_or_default(),
                    context,
                );
                retry_policy.close(context);
                context.set_attribute(retry_context_constants::CLOSED, true.into());
            }
        } else {
            retry_policy.close(context);
            context.set_attribute(retry_context_constants::CLOSED, true.into());
        }
    }

    /// Records a failure with the policy and, for a stateful retry, in the cache.
    fn register_throwable(
        &self,
        retry_policy: &dyn RetryPolicy,
        state: Option<&dyn RetryState>,
        context: Arc<dyn RetryContext>,
        error: &RetryError,
    ) -> Result<(), RetryError> {
        retry_policy.register_error(context.as_ref(), error);
        self.register_context(context, state)
    }

    /// Stores the context of a stateful retry under its key.
    fn register_context(
        &self,
        context: Arc<dyn RetryContext>,
        state: Option<&dyn RetryState>,
    ) -> Result<(), RetryError> {
        let Some(state) = state else {
            return Ok(());
        };
        let Some(key) = state.get_key() else {
            return Ok(());
        };

        if context.get_retry_count() > 1
            && !self
                .retry_context_cache
                .contains_key_for(key.as_str(), context.as_ref())
        {
            return Err(RetryError::retry(
                "Inconsistent state for failed item key: cache key has changed. Consider whether equality for the key is inconsistent, or if you need to supply a better key",
            ));
        }

        self.retry_context_cache.put(key.as_str(), context)
    }

    /// Returns the context for a retry block, reusing a stored one when possible.
    fn open(
        &self,
        retry_policy: &dyn RetryPolicy,
        state: Option<&dyn RetryState>,
    ) -> Result<Arc<dyn RetryContext>, RetryError> {
        let Some(state) = state else {
            return self.do_open_internal(retry_policy, None);
        };

        let key = state.get_key().map(String::as_str).unwrap_or_default();

        if state.is_force_refresh() {
            return self.do_open_internal(retry_policy, Some(state));
        }

        // Without a cache hit the expensive rehydration of the context is skipped entirely.
        if !self.retry_context_cache.contains_key(key) {
            return self.do_open_internal(retry_policy, Some(state));
        }

        if let Some(context) = self.retry_context_cache.get(key) {
            // Start with a clean slate for state that others may be inspecting. Every block flag
            // is cleared, not just the three an outcome needs: a cancelled or vetoed block leaves
            // ABORTED/TERMINATED on the context, and a cached context is reused by the next
            // invocation of the same item.
            clear_block_flags(context.as_ref());
            Ok(context)
        } else {
            if self.retry_context_cache.contains_key(key) {
                return Err(RetryError::retry(
                    "Inconsistent state for failed item: no history found. Consider whether equality for the item is inconsistent, or if you need to supply a better key generator",
                ));
            }
            // The cache can expire between the two lookups, so a fresh context is the only
            // consistent answer here.
            self.do_open_internal(retry_policy, Some(state))
        }
    }

    /// Creates a context and, for a global state, stores it immediately.
    fn do_open_internal(
        &self,
        retry_policy: &dyn RetryPolicy,
        state: Option<&dyn RetryState>,
    ) -> Result<Arc<dyn RetryContext>, RetryError> {
        let parent = RetrySynchronizationManager::get_context();
        let context = retry_policy.open(parent);

        if let Some(state) = state {
            let key = state
                .get_key()
                .map(|key| AnyValue::String(key.clone()))
                .unwrap_or(AnyValue::Null);
            context.set_attribute(retry_context_constants::STATE_KEY, key);
        }

        if context.flag(retry_context_constants::GLOBAL_STATE)
            && let Some(key) = state.and_then(|state| state.get_key())
        {
            self.retry_context_cache
                .put(key.as_str(), context.clone())?;
        }

        Ok(context)
    }

    /// Produces the result of an exhausted retry block.
    async fn handle_retry_exhausted<T>(
        &self,
        recovery_callback: Option<&dyn RecoveryCallback<T>>,
        context: &dyn RetryContext,
        state: Option<&dyn RetryState>,
    ) -> Result<T, RetryError> {
        context.set_attribute(retry_context_constants::EXHAUSTED, true.into());
        if let Some(state) = state {
            self.retry_context_cache.remove(
                state.get_key().map(String::as_str).unwrap_or_default(),
                context,
            );
        }

        let do_recover = !context.is_no_recovery();
        if let Some(recovery_callback) = recovery_callback {
            if do_recover {
                let recovered = recovery_callback.recover(context).await?;
                context.set_attribute(retry_context_constants::RECOVERED, true.into());
                return Ok(recovered);
            }
            debug!("Retry exhausted and recovery is disabled for this error");
        }

        if state.is_some() {
            debug!("Retry exhausted after the last attempt with no recovery path");
            return Err(self.rethrow(
                context,
                "Retry exhausted after last attempt with no recovery path",
                self.throw_last_exception_on_exhausted || !do_recover,
            ));
        }

        match context.get_last_error() {
            Some(error) => Err(error),
            None => Err(RetryError::retry("Exception in retry")),
        }
    }

    /// Builds the error reported when a stateful retry is exhausted.
    fn rethrow(&self, context: &dyn RetryContext, msg: &str, wrap: bool) -> RetryError {
        let last_error = context.get_last_error();

        if wrap {
            return match last_error {
                Some(error) => error,
                None => RetryError::retry(msg),
            };
        }

        match last_error {
            Some(error) => RetryError::exhausted_with_cause(msg, Box::new(error)),
            None => RetryError::exhausted(msg),
        }
    }

    fn do_open_interceptors(&self, context: &dyn RetryContext) -> bool {
        self.listeners.iter().all(|listener| listener.open(context))
    }

    fn do_close_interceptors(&self, context: &dyn RetryContext, last_error: Option<&dyn AnyError>) {
        for listener in self.listeners.iter().rev() {
            listener.close(context, last_error);
        }
    }

    fn do_on_success_interceptors<T>(&self, context: &dyn RetryContext, result: &T)
    where
        T: Any,
    {
        for listener in self.listeners.iter().rev() {
            listener.on_success(context, result);
        }
    }

    fn do_on_error_interceptors(&self, context: &dyn RetryContext, error: &RetryError) {
        for listener in self.listeners.iter().rev() {
            listener.on_error(context, error);
        }
    }

    /// Returns whether a failed attempt must be rethrown instead of retried.
    fn should_rethrow(
        &self,
        _retry_policy: &dyn RetryPolicy,
        context: &dyn RetryContext,
        state: Option<&dyn RetryState>,
    ) -> bool {
        let Some(state) = state else {
            return false;
        };
        match context.get_last_error() {
            Some(error) => state.rollback_for(&error),
            None => false,
        }
    }
}

/// Removes the flags a previous block may have left on a reused context.
fn clear_block_flags(context: &dyn RetryContext) {
    for flag in [
        retry_context_constants::CLOSED,
        retry_context_constants::EXHAUSTED,
        retry_context_constants::RECOVERED,
        retry_context_constants::ABORTED,
        retry_context_constants::TERMINATED,
    ] {
        context.remove_attribute(flag);
    }
}

/// Finishes a retry block exactly once, whatever ends it.
///
/// The original implementation uses a `finally` block. Rust has no `finally`, so the cleanup
/// lives in [`Drop`] instead: it runs when the enclosing block returns, when it returns early
/// through `?`, when a callback panics, and when the future is dropped because the caller was
/// cancelled. That last case matters in a server: a disconnected client drops the future, and
/// policies, listeners and the cached context must still be released.
struct ExecutionGuard<'a> {
    template: &'a RetryTemplate,
    retry_policy: &'a dyn RetryPolicy,
    context: Arc<dyn RetryContext>,
    state: Option<&'a dyn RetryState>,
    last_error: Mutex<Option<RetryError>>,
    exhausted: AtomicBool,
    completed: AtomicBool,
    started: Instant,
}

impl<'a> ExecutionGuard<'a> {
    fn new(
        template: &'a RetryTemplate,
        retry_policy: &'a dyn RetryPolicy,
        context: Arc<dyn RetryContext>,
        state: Option<&'a dyn RetryState>,
    ) -> Self {
        Self {
            template,
            retry_policy,
            context,
            state,
            last_error: Mutex::new(None),
            exhausted: AtomicBool::new(false),
            completed: AtomicBool::new(false),
            started: Instant::now(),
        }
    }

    /// Forgets the previous failure, so a success does not look like a failed block.
    fn clear_error(&self) {
        if let Ok(mut last_error) = self.last_error.lock() {
            *last_error = None;
        }
    }

    /// Remembers the failure of the current attempt.
    fn record_error(&self, error: &RetryError) {
        if let Ok(mut last_error) = self.last_error.lock() {
            *last_error = Some(error.clone());
        }
    }

    /// Records that the block ended by exhausting its attempts.
    fn mark_exhausted(&self) {
        self.exhausted.store(true, Ordering::Release);
    }

    /// Records that the block reached one of its normal exits.
    ///
    /// A block that never calls this was ended from outside — a panic or a cancellation — and is
    /// reported as aborted instead of success.
    fn mark_completed(&self) {
        self.completed.store(true, Ordering::Release);
    }

    /// Runs the cleanup steps the template owes the policies and the listeners.
    fn run(&self) {
        if !self.completed.load(Ordering::Acquire) {
            self.context
                .set_attribute(retry_context_constants::ABORTED, true.into());
        }

        // The block owns the clock: a context can be shared by concurrent blocks, so the elapsed
        // time is measured here and published only for the close hooks to read.
        self.context.set_attribute(
            retry_context_constants::ELAPSED,
            AnyValue::BoxedValue(Box::new(self.started.elapsed())),
        );

        let last_error = self
            .last_error
            .lock()
            .ok()
            .and_then(|last_error| last_error.clone());
        let succeeded = last_error.is_none() || self.exhausted.load(Ordering::Acquire);

        self.template.close(
            self.retry_policy,
            self.context.as_ref(),
            self.state,
            succeeded,
        );
        self.template.do_close_interceptors(
            self.context.as_ref(),
            last_error.as_ref().map(|error| error as &dyn AnyError),
        );

        // The duration belongs to this block only; leaving it behind would let a later block on the
        // same context report it.
        self.context
            .remove_attribute(retry_context_constants::ELAPSED);
    }
}

impl Drop for ExecutionGuard<'_> {
    fn drop(&mut self) {
        // A panicking close hook must not turn a single failure into an abort: during an
        // unwind a second panic would terminate the process. The panic is reported instead.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.run()));
        if outcome.is_err() {
            error!("A retry close hook panicked while cleaning up the retry block");
        }
    }
}

impl Default for RetryTemplate {
    fn default() -> Self {
        Self {
            back_off_policy: Arc::new(NoBackOffPolicy::new()),
            retry_policy: Arc::new(SimpleRetryPolicy::with_max_attempts(3)),
            listeners: Vec::new(),
            retry_context_cache: CompositeRetryContextCache::default(),
            throw_last_exception_on_exhausted: false,
        }
    }
}

#[async_trait]
impl<T> RetryOperations<T> for RetryTemplate
where
    T: Send + 'static,
{
    async fn execute(&self, retry_callback: impl RetryCallback<T>) -> Result<T, RetryError> {
        self.do_execute(retry_callback, None, None).await
    }

    async fn execute_with_recovery(
        &self,
        retry_callback: impl RetryCallback<T>,
        recovery_callback: &dyn RecoveryCallback<T>,
    ) -> Result<T, RetryError> {
        self.do_execute(retry_callback, Some(recovery_callback), None)
            .await
    }

    async fn execute_with_state(
        &self,
        retry_callback: impl RetryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, RetryError> {
        self.do_execute(retry_callback, None, Some(state)).await
    }

    async fn execute_with_all(
        &self,
        retry_callback: impl RetryCallback<T>,
        recovery_callback: &dyn RecoveryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, RetryError> {
        self.do_execute(retry_callback, Some(recovery_callback), Some(state))
            .await
    }
}

/// The pair of caches a template keeps: one for stateful retries, one for global state.
///
/// A circuit-breaker context is marked global and must never be removed when a retry closes,
/// so it lives in a separate cache. Looking a key up consults both, and storing picks the
/// cache by the context's flag.
#[derive(Clone)]
struct CompositeRetryContextCache {
    stateful: Arc<dyn RetryContextCache>,
    circuit_breaker: Arc<dyn RetryContextCache>,
}

impl CompositeRetryContextCache {
    fn get(&self, key: &str) -> Option<Arc<dyn RetryContext>> {
        self.stateful
            .get(key)
            .or_else(|| self.circuit_breaker.get(key))
    }

    fn put(&self, key: &str, context: Arc<dyn RetryContext>) -> Result<(), RetryError> {
        if context.flag(retry_context_constants::GLOBAL_STATE) {
            self.circuit_breaker.put(key, context)
        } else {
            self.stateful.put(key, context)
        }
    }

    fn remove(&self, key: &str, context: &dyn RetryContext) {
        if !context.flag(retry_context_constants::GLOBAL_STATE) {
            self.stateful.remove(key);
        }
    }

    fn contains_key(&self, key: &str) -> bool {
        self.stateful.contains_key(key) || self.circuit_breaker.contains_key(key)
    }

    fn contains_key_for(&self, key: &str, context: &dyn RetryContext) -> bool {
        if context.flag(retry_context_constants::GLOBAL_STATE) {
            self.circuit_breaker.contains_key(key)
        } else {
            self.stateful.contains_key(key)
        }
    }
}

impl Default for CompositeRetryContextCache {
    fn default() -> Self {
        Self {
            stateful: Arc::new(MapRetryContextCache::with_capacity_and_eviction(
                crate::policy::retry_context_cache::DEFAULT_CAPACITY,
                true,
            )),
            circuit_breaker: Arc::new(MapRetryContextCache::with_capacity_and_eviction(
                crate::policy::retry_context_cache::DEFAULT_CAPACITY,
                false,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::RetryTemplate;

    #[test]
    fn clones_share_the_expensive_state() {
        let template = RetryTemplate::builder().max_attempts(3).build();
        let cloned = template.clone();

        assert!(Arc::ptr_eq(&template.retry_policy, &cloned.retry_policy));
        assert!(Arc::ptr_eq(
            &template.back_off_policy,
            &cloned.back_off_policy
        ));
    }

    #[test]
    fn the_default_template_retries_three_times_without_waiting() {
        let template = RetryTemplate::default();

        assert_eq!(template.retry_policy().get_max_attempts(), 3);
        assert!(!template.has_listeners());
    }
}
