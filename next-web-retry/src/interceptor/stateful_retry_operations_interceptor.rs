//! Retries a call across invocations, remembering the item that failed.

use std::{future::Future, sync::Arc};

use next_web_core::any::AnyValue;
use tracing::debug;

use crate::{
    Predicate,
    error::retry_error::RetryError,
    interceptor::{
        method_arguments_key_generator::{MethodArgumentsKeyGenerator, default_key},
        method_invocation::MethodInvocation,
        method_invocation_recoverer::MethodInvocationRecoverer,
        method_invocation_retry_callback::MethodInvocationRetryCallback,
        new_method_arguments_identifier::NewMethodArgumentsIdentifier,
    },
    policy::never_retry_policy::NeverRetryPolicy,
    retry_operations::RetryOperations,
    support::{default_retry_state::DefaultRetryState, retry_template::RetryTemplate},
};

/// Retries a call across invocations, remembering the item that failed.
///
/// This is the stateful interceptor: a failed call rethrows instead of looping, and the item is
/// remembered under a key so the next invocation for the same item resumes with the stored
/// attempt count. That is what lets a retry leave a transaction boundary: each attempt, and the
/// recovery, runs in a fresh transaction.
///
/// By default the key is derived from the call arguments. A
/// [`MethodArgumentsKeyGenerator`] can override it, and returning `None` from it disables
/// retrying for the call.
pub struct StatefulRetryOperationsInterceptor<T> {
    retry_operations: RetryTemplate,
    key_generator: Option<Arc<dyn MethodArgumentsKeyGenerator>>,
    recoverer: Option<Arc<dyn MethodInvocationRecoverer<T>>>,
    new_arguments_identifier: Option<Arc<dyn NewMethodArgumentsIdentifier>>,
    label: Option<String>,
    rollback_classifier: Option<Arc<dyn Predicate<RetryError>>>,
    use_raw_key: bool,
}

impl<T> StatefulRetryOperationsInterceptor<T> {
    /// Creates an interceptor from fully configured retry operations.
    pub fn new(retry_operations: RetryTemplate) -> Self {
        Self {
            retry_operations,
            key_generator: None,
            recoverer: None,
            new_arguments_identifier: None,
            label: None,
            rollback_classifier: None,
            use_raw_key: false,
        }
    }

    /// Creates an interceptor that performs exactly one attempt per invocation.
    ///
    /// A stateful retry relies on the caller presenting the item again, so the default policy
    /// never retries in place.
    pub fn default_one_attempt() -> Self {
        let mut template = RetryTemplate::default();
        template.set_retry_policy(NeverRetryPolicy::new());
        Self::new(template)
    }

    /// Replaces the retry operations.
    pub fn set_retry_operations(&mut self, retry_operations: RetryTemplate) {
        self.retry_operations = retry_operations;
    }

    /// Installs the key generator.
    pub fn set_key_generator(&mut self, key_generator: Arc<dyn MethodArgumentsKeyGenerator>) {
        self.key_generator = Some(key_generator);
    }

    /// Installs the recoverer used when the retry is exhausted.
    pub fn set_recoverer(&mut self, recoverer: Arc<dyn MethodInvocationRecoverer<T>>) {
        self.recoverer = Some(recoverer);
    }

    /// Installs the identifier that recognises never-seen arguments.
    pub fn set_new_arguments_identifier(
        &mut self,
        identifier: Arc<dyn NewMethodArgumentsIdentifier>,
    ) {
        self.new_arguments_identifier = Some(identifier);
    }

    /// Sets the reporting label.
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = Some(label.into());
    }

    /// Sets the rollback classifier.
    ///
    /// The classifier answers `true` for failures that must be rethrown. Without it every
    /// failure is rethrown, which is the conservative default.
    pub fn set_rollback_classifier(&mut self, classifier: Arc<dyn Predicate<RetryError>>) {
        self.rollback_classifier = Some(classifier);
    }

    /// Uses the raw generated key instead of a key qualified by the call name.
    ///
    /// Only safe when the generated key is unique across every intercepted call.
    pub fn set_use_raw_key(&mut self, use_raw_key: bool) {
        self.use_raw_key = use_raw_key;
    }

    /// Returns the retry operations of this interceptor.
    pub fn retry_operations(&self) -> &RetryTemplate {
        &self.retry_operations
    }
}

impl<T: Send + Sync + 'static> StatefulRetryOperationsInterceptor<T> {
    /// Invokes the call once, resuming the retry of the same item if it failed before.
    pub async fn invoke<F, Fut>(
        &self,
        invocation: MethodInvocation,
        proceed: F,
    ) -> Result<T, RetryError>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, RetryError>> + Send,
    {
        let invocation = match &self.label {
            Some(label) => invocation.with_label(label.clone()),
            None => invocation,
        };

        let arguments = invocation.arguments().to_vec();
        let key = self.create_key(&invocation);
        let force_refresh = self
            .new_arguments_identifier
            .as_ref()
            .map(|identifier| identifier.is_new(&arguments))
            .unwrap_or(false);

        let state = DefaultRetryState::with_optional_key_and_rollback_classifier(
            key,
            force_refresh,
            self.rollback_classifier.clone(),
        );

        if let Some(recoverer) = &self.recoverer {
            let recovery = ItemRecovererCallback {
                arguments,
                recoverer: recoverer.clone(),
            };
            let callback = MethodInvocationRetryCallback::new(invocation, proceed);
            self.retry_operations
                .execute_with_all(callback, &recovery, &state)
                .await
        } else {
            let callback = MethodInvocationRetryCallback::new(invocation, proceed);
            self.retry_operations
                .execute_with_state(callback, &state)
                .await
        }
    }

    /// Builds the cache key of a call.
    ///
    /// A key generator that returns `None` means the call must not be retried at all, so the
    /// key is `None` and the retry context is never cached.
    fn create_key(&self, invocation: &MethodInvocation) -> Option<String> {
        let generated = match &self.key_generator {
            Some(generator) => generator.get_key(invocation.arguments()),
            None => default_key(invocation.arguments()),
        };
        let generated = generated?;

        if self.use_raw_key {
            return Some(generated);
        }

        let name = invocation.label();
        let key = format!("{name}:{generated}");
        debug!(key = %key, "Stateful retry key created");
        Some(key)
    }
}

impl<T> Default for StatefulRetryOperationsInterceptor<T> {
    fn default() -> Self {
        Self::default_one_attempt()
    }
}

/// Recovers a stateful call through its [`MethodInvocationRecoverer`].
struct ItemRecovererCallback<T> {
    arguments: Vec<AnyValue>,
    recoverer: Arc<dyn MethodInvocationRecoverer<T>>,
}

#[next_web_core::async_trait]
impl<T> crate::recovery_callback::RecoveryCallback<T> for ItemRecovererCallback<T>
where
    T: Send + Sync + 'static,
{
    async fn recover(
        &self,
        context: &dyn crate::retry_context::RetryContext,
    ) -> Result<T, RetryError> {
        let cause = context.get_last_error();
        self.recoverer.recover(&self.arguments, cause.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicU16, Ordering},
    };

    use next_web_core::any::AnyValue;

    use super::StatefulRetryOperationsInterceptor;
    use crate::{
        error::retry_error::RetryError,
        interceptor::{
            method_invocation::MethodInvocation, method_invocation_recoverer::ConstantRecoverer,
        },
    };

    fn invocation() -> MethodInvocation {
        MethodInvocation::new("Service::call", vec![AnyValue::from("item-1")])
    }

    #[tokio::test]
    async fn a_failure_is_rethrown_after_one_attempt() {
        let interceptor = StatefulRetryOperationsInterceptor::default_one_attempt();
        let attempts = Arc::new(AtomicU16::new(0));
        let attempts_in_call = attempts.clone();

        let result = interceptor
            .invoke(invocation(), move || {
                let attempts = attempts_in_call.clone();
                async move {
                    attempts.fetch_add(1, Ordering::Relaxed);
                    Err::<u64, _>(RetryError::custom("fatal"))
                }
            })
            .await;

        assert_eq!(
            result.expect_err("the call should fail"),
            RetryError::custom("fatal")
        );
        assert_eq!(attempts.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn the_next_invocation_resumes_and_does_not_call_again() {
        let interceptor = StatefulRetryOperationsInterceptor::default_one_attempt();
        let attempts = Arc::new(AtomicU16::new(0));

        let first = interceptor
            .invoke(invocation(), {
                let attempts = attempts.clone();
                move || {
                    let attempts = attempts.clone();
                    async move {
                        attempts.fetch_add(1, Ordering::Relaxed);
                        Err::<u64, _>(RetryError::custom("fatal"))
                    }
                }
            })
            .await;
        assert!(first.is_err());
        assert_eq!(attempts.load(Ordering::Relaxed), 1);

        let second = interceptor
            .invoke(invocation(), {
                let attempts = attempts.clone();
                move || {
                    let attempts = attempts.clone();
                    async move {
                        attempts.fetch_add(1, Ordering::Relaxed);
                        Ok::<u64, RetryError>(1)
                    }
                }
            })
            .await;

        assert!(matches!(second, Err(RetryError::ExhaustedRetryError(_))));
        assert_eq!(
            attempts.load(Ordering::Relaxed),
            1,
            "the resumed invocation must not run the callback again"
        );
    }

    #[tokio::test]
    async fn a_key_generator_returning_no_key_disables_caching() {
        let mut interceptor = StatefulRetryOperationsInterceptor::default_one_attempt();
        interceptor.set_key_generator(Arc::new(|_: &[AnyValue]| None));
        let calls = Arc::new(AtomicU16::new(0));

        let result = interceptor
            .invoke(invocation(), {
                let calls = calls.clone();
                move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::Relaxed);
                        Err::<u64, _>(RetryError::custom("fatal"))
                    }
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn the_recoverer_returns_a_value_when_the_retry_is_exhausted() {
        let mut interceptor = StatefulRetryOperationsInterceptor::default_one_attempt();
        interceptor.set_recoverer(Arc::new(ConstantRecoverer::new(7_u64)));

        // A stateful retry rethrows the first failure, so the recovery path is reached by the
        // next invocation for the same item.
        let first = interceptor
            .invoke(invocation(), || async {
                Err::<u64, _>(RetryError::custom("fatal"))
            })
            .await;
        assert!(first.is_err());

        let recovered = interceptor
            .invoke(invocation(), || async { Ok::<u64, RetryError>(0) })
            .await;

        assert_eq!(recovered.expect("the exhausted retry should recover"), 7);
    }

    #[tokio::test]
    async fn a_label_replaces_the_method_name_in_the_context() {
        let mut interceptor = StatefulRetryOperationsInterceptor::default_one_attempt();
        interceptor.set_label("orders");

        let label = Arc::new(std::sync::Mutex::new(None));
        let label_in_call = label.clone();
        let _ = interceptor
            .invoke(invocation(), move || {
                let label = label_in_call.clone();
                async move {
                    if let Some(context) =
                        crate::support::retry_synchronization_manager::RetrySynchronizationManager::get_context()
                        && let Ok(mut seen) = label.lock()
                    {
                        *seen = context.get_name();
                    }
                    Err::<u64, _>(RetryError::custom("fatal"))
                }
            })
            .await;

        assert_eq!(
            label.lock().ok().and_then(|label| label.clone()),
            Some("orders".to_owned())
        );
    }
}
