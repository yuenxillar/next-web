//! Retries a call in place until it succeeds.

use std::{future::Future, sync::Arc};

use next_web_core::{any::AnyValue, async_trait};

use crate::{
    error::retry_error::RetryError,
    interceptor::{
        method_invocation::MethodInvocation,
        method_invocation_recoverer::MethodInvocationRecoverer,
        method_invocation_retry_callback::MethodInvocationRetryCallback,
    },
    recovery_callback::RecoveryCallback,
    retry_context::RetryContext,
    retry_operations::RetryOperations,
    support::retry_template::RetryTemplate,
};

/// Retries a call in place until it succeeds or the retry policy gives up.
///
/// This is the stateless interceptor: the caller blocks inside [`RetryOperationsInterceptor::invoke`]
/// while the template runs the call again. The intercepted call is described by a
/// [`MethodInvocation`] and performed by the closure passed to `invoke`, which is the Rust
/// counterpart of a reflective method invocation.
///
/// Every attempt publishes the call description on the retry context, so listeners and metrics
/// can report which call failed.
pub struct RetryOperationsInterceptor<T> {
    retry_operations: RetryTemplate,
    recoverer: Option<Arc<dyn MethodInvocationRecoverer<T>>>,
    label: Option<String>,
}

impl<T> RetryOperationsInterceptor<T> {
    /// Creates an interceptor with the default retry behaviour.
    pub fn new(retry_operations: RetryTemplate) -> Self {
        Self {
            retry_operations,
            recoverer: None,
            label: None,
        }
    }

    /// Replaces the retry operations.
    pub fn set_retry_operations(&mut self, retry_operations: RetryTemplate) {
        self.retry_operations = retry_operations;
    }

    /// Installs a recoverer used when the retry is exhausted.
    pub fn set_recoverer(&mut self, recoverer: Arc<dyn MethodInvocationRecoverer<T>>) {
        self.recoverer = Some(recoverer);
    }

    /// Sets the reporting label of the intercepted call.
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = Some(label.into());
    }

    /// Returns the retry operations of this interceptor.
    pub fn retry_operations(&self) -> &RetryTemplate {
        &self.retry_operations
    }

    /// Returns the arguments of a call, published on the retry context for listeners.
    pub fn arguments_of(invocation: &MethodInvocation) -> &[AnyValue] {
        invocation.arguments()
    }
}

impl<T: Send + Sync + 'static> RetryOperationsInterceptor<T> {
    /// Invokes the call, retrying according to the configured template.
    ///
    /// `proceed` performs one attempt. When a recoverer is configured it produces the value of
    /// the interceptor once the retry is exhausted; otherwise the last failure is returned.
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

        if let Some(recoverer) = &self.recoverer {
            let recovery = ItemRecovererCallback {
                arguments: invocation.arguments().to_vec(),
                recoverer: recoverer.clone(),
            };
            let callback = MethodInvocationRetryCallback::new(invocation, proceed);
            self.retry_operations
                .execute_with_recovery(callback, &recovery)
                .await
        } else {
            let callback = MethodInvocationRetryCallback::new(invocation, proceed);
            self.retry_operations.execute(callback).await
        }
    }
}

impl<T> Default for RetryOperationsInterceptor<T> {
    fn default() -> Self {
        Self::new(RetryTemplate::default())
    }
}

/// Recovers an intercepted call through its [`MethodInvocationRecoverer`].
struct ItemRecovererCallback<T> {
    arguments: Vec<AnyValue>,
    recoverer: Arc<dyn MethodInvocationRecoverer<T>>,
}

#[async_trait]
impl<T> RecoveryCallback<T> for ItemRecovererCallback<T>
where
    T: Send + Sync + 'static,
{
    async fn recover(&self, context: &dyn RetryContext) -> Result<T, RetryError> {
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

    use super::RetryOperationsInterceptor;
    use crate::{
        error::retry_error::RetryError,
        interceptor::{
            method_invocation::MethodInvocation, method_invocation_recoverer::ConstantRecoverer,
        },
        support::retry_template::RetryTemplate,
    };

    #[tokio::test]
    async fn the_call_is_retried_until_it_succeeds() {
        let interceptor =
            RetryOperationsInterceptor::new(RetryTemplate::builder().max_attempts(3).build());
        let attempts = Arc::new(AtomicU16::new(0));
        let attempts_in_call = attempts.clone();

        let result = interceptor
            .invoke(
                MethodInvocation::new("Service::call", vec![AnyValue::from(1_u64)]),
                move || {
                    let attempts = attempts_in_call.clone();
                    async move {
                        let attempt = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                        if attempt < 3 {
                            return Err(RetryError::custom("transient"));
                        }
                        Ok(attempt)
                    }
                },
            )
            .await;

        assert_eq!(result.expect("the call should succeed"), 3);
        assert_eq!(attempts.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn the_recoverer_produces_the_value_on_exhaustion() {
        let mut interceptor =
            RetryOperationsInterceptor::new(RetryTemplate::builder().max_attempts(1).build());
        interceptor.set_recoverer(Arc::new(ConstantRecoverer::new(42_u64)));

        let result = interceptor
            .invoke(
                MethodInvocation::without_arguments("Service::call"),
                || async { Err::<u64, _>(RetryError::custom("fatal")) },
            )
            .await;

        assert_eq!(result.expect("recovery should succeed"), 42);
    }

    #[tokio::test]
    async fn without_a_recoverer_the_last_failure_is_returned() {
        let interceptor =
            RetryOperationsInterceptor::new(RetryTemplate::builder().max_attempts(1).build());

        let result = interceptor
            .invoke(
                MethodInvocation::without_arguments("Service::call"),
                || async { Err::<u64, _>(RetryError::custom("fatal")) },
            )
            .await;

        assert_eq!(
            result.expect_err("the call should fail"),
            RetryError::custom("fatal")
        );
    }
}
