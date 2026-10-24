//! The retry callback that performs an intercepted call and publishes its description.

use std::marker::PhantomData;

use next_web_core::{any::AnyValue, async_trait};

use crate::{
    error::retry_error::RetryError,
    interceptor::method_invocation::MethodInvocation,
    retry_callback::RetryCallback,
    retry_context::{RetryContext, retry_context_constants},
    support::args::Args,
};

/// The retry context attribute that holds the name of the intercepted call.
pub const METHOD: &str = "method";

/// The retry context attribute that holds the [`Args`] of the intercepted call.
pub const METHOD_ARGS: &str = "methodArgs";

/// The retry context attribute kept for compatibility with the original argument name.
pub const ARGS: &str = "ARGS";

/// A retry callback that performs an intercepted call.
///
/// Besides calling the closure, it publishes the call description on the context: the label,
/// the method name and the arguments. That is what lets a listener or a metrics recorder report
/// which call is being retried without receiving the callback itself.
pub struct MethodInvocationRetryCallback<T, F> {
    invocation: MethodInvocation,
    proceed: F,
    marker: PhantomData<fn() -> T>,
}

impl<T, F> MethodInvocationRetryCallback<T, F> {
    /// Creates a callback for a call described by `invocation` and performed by `proceed`.
    pub fn new(invocation: MethodInvocation, proceed: F) -> Self {
        Self {
            invocation,
            proceed,
            marker: PhantomData,
        }
    }

    /// Returns the description of the intercepted call.
    pub fn invocation(&self) -> &MethodInvocation {
        &self.invocation
    }
}

#[async_trait]
impl<T, F, Fut> RetryCallback<T> for MethodInvocationRetryCallback<T, F>
where
    T: Send + 'static,
    F: Fn() -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<T, RetryError>> + Send,
{
    async fn do_with_retry(&self, context: &dyn RetryContext) -> Result<T, RetryError> {
        context.set_attribute(
            retry_context_constants::NAME,
            self.invocation.label().into(),
        );
        context.set_attribute(METHOD, self.invocation.method().into());
        let args = Args::new(self.invocation.arguments().to_vec());
        context.set_attribute(METHOD_ARGS, AnyValue::BoxedValue(Box::new(args.clone())));
        context.set_attribute(ARGS, AnyValue::BoxedValue(Box::new(args)));

        (self.proceed)().await
    }

    fn label(&self) -> Option<&str> {
        Some(self.invocation.label())
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::any::AnyValue;

    use super::{METHOD, METHOD_ARGS, MethodInvocationRetryCallback};
    use crate::{
        error::retry_error::RetryError, interceptor::method_invocation::MethodInvocation,
        retry_callback::RetryCallback, retry_context::SyncAttributeAccessor, support::args::Args,
    };

    #[tokio::test]
    async fn the_callback_publishes_the_call_description_and_runs_it() {
        let invocation =
            MethodInvocation::new("Service::call", vec![AnyValue::from(7_u64)]).with_label("svc");
        let callback = MethodInvocationRetryCallback::new(invocation, || async {
            Ok::<_, RetryError>(42_u64)
        });
        let context = crate::context::retry_context_support::RetryContextSupport::new();

        let result = callback
            .do_with_retry(&context)
            .await
            .expect("the callback should succeed");

        assert_eq!(result, 42);
        assert_eq!(callback.label(), Some("svc"));
        assert_eq!(
            context
                .get_attribute(METHOD)
                .and_then(|value| value.as_string()),
            Some("Service::call".to_owned())
        );
        let args = context
            .get_attribute(METHOD_ARGS)
            .and_then(|value| value.as_ref_value::<Args>().cloned())
            .expect("the arguments should be published");
        assert_eq!(args.len(), 1);
    }
}
