//! A listener adapter that receives the intercepted call description.

use std::any::Any;

use crate::{
    interceptor::{METHOD, METHOD_ARGS, method_invocation::MethodInvocation},
    retry_context::RetryContext,
    support::args::Args,
};

/// A listener that receives the call description as well as the retry context.
///
/// The original listens for callbacks that happen to be method invocations. The crate stores
/// the call description in the retry context instead, so this trait describes the extra hooks a
/// listener gets when it cares about the intercepted call. [`MethodInvocationRetryListenerSupport`]
/// adapts it to a plain [`RetryListener`](crate::retry_listener::RetryListener).
pub trait MethodInvocationRetryListener
where
    Self: Send + Sync,
{
    /// Called before the first attempt. Returning `false` terminates the retry.
    fn do_open(&self, _context: &dyn RetryContext, _invocation: &MethodInvocation) -> bool {
        true
    }

    /// Called after the last attempt.
    fn do_close(
        &self,
        _context: &dyn RetryContext,
        _invocation: &MethodInvocation,
        _error: Option<&dyn next_web_core::any::AnyError>,
    ) {
    }

    /// Called after a successful attempt.
    fn do_on_success(
        &self,
        _context: &dyn RetryContext,
        _invocation: &MethodInvocation,
        _result: &dyn Any,
    ) {
    }

    /// Called after every failed attempt.
    fn do_on_error(
        &self,
        _context: &dyn RetryContext,
        _invocation: &MethodInvocation,
        _error: &dyn next_web_core::any::AnyError,
    ) {
    }
}

/// Adapts a [`MethodInvocationRetryListener`] to a
/// [`RetryListener`](crate::retry_listener::RetryListener).
///
/// Every hook reads the intercepted call from the retry context and forwards it, so a listener
/// only runs its method-invocation hooks for retries that came from an interceptor. A retry that
/// has no call description is ignored, which is the behaviour the original gets from its
/// `instanceof` checks.
pub struct MethodInvocationRetryListenerSupport<L> {
    listener: L,
}

impl<L> MethodInvocationRetryListenerSupport<L> {
    /// Wraps a listener.
    pub fn new(listener: L) -> Self {
        Self { listener }
    }

    /// Returns the wrapped listener.
    pub fn listener(&self) -> &L {
        &self.listener
    }
}

/// Rebuilds the call description from the retry context.
fn invocation_from_context(context: &dyn RetryContext) -> Option<MethodInvocation> {
    let method = context
        .get_attribute(METHOD)
        .and_then(|value| value.as_string())?;
    let arguments = context
        .get_attribute(METHOD_ARGS)
        .and_then(|value| value.as_ref_value::<Args>().cloned())
        .map(|args| args.get_args().to_vec())
        .unwrap_or_default();
    let label = context.get_name();

    let invocation = MethodInvocation::new(method, arguments);
    Some(match label {
        Some(label) => invocation.with_label(label),
        None => invocation,
    })
}

impl<L> crate::retry_listener::RetryListener for MethodInvocationRetryListenerSupport<L>
where
    L: MethodInvocationRetryListener,
{
    fn open(&self, context: &dyn RetryContext) -> bool {
        match invocation_from_context(context) {
            Some(invocation) => self.listener.do_open(context, &invocation),
            None => true,
        }
    }

    fn close(&self, context: &dyn RetryContext, error: Option<&dyn next_web_core::any::AnyError>) {
        if let Some(invocation) = invocation_from_context(context) {
            self.listener.do_close(context, &invocation, error);
        }
    }

    fn on_success(&self, context: &dyn RetryContext, result: &dyn Any) {
        if let Some(invocation) = invocation_from_context(context) {
            self.listener.do_on_success(context, &invocation, result);
        }
    }

    fn on_error(&self, context: &dyn RetryContext, error: &dyn next_web_core::any::AnyError) {
        if let Some(invocation) = invocation_from_context(context) {
            self.listener.do_on_error(context, &invocation, error);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicU16, Ordering},
    };

    use next_web_core::any::AnyValue;

    use super::{MethodInvocationRetryListener, MethodInvocationRetryListenerSupport};
    use crate::{
        interceptor::{METHOD, METHOD_ARGS, method_invocation::MethodInvocation},
        retry_context::{RetryContext, SyncAttributeAccessor},
        retry_listener::RetryListener,
        support::args::Args,
    };

    #[derive(Default)]
    struct CountingListener {
        opens: AtomicU16,
        errors: AtomicU16,
    }

    impl MethodInvocationRetryListener for CountingListener {
        fn do_open(&self, _context: &dyn RetryContext, _invocation: &MethodInvocation) -> bool {
            self.opens.fetch_add(1, Ordering::Relaxed);
            true
        }

        fn do_on_error(
            &self,
            _context: &dyn RetryContext,
            _invocation: &MethodInvocation,
            _error: &dyn next_web_core::any::AnyError,
        ) {
            self.errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn the_adapter_forwards_to_an_intercepted_call() {
        let context = crate::context::retry_context_support::RetryContextSupport::new();
        context.set_attribute(METHOD, "Service::call".into());
        context.set_attribute(
            METHOD_ARGS,
            AnyValue::BoxedValue(Box::new(Args::new(vec![AnyValue::from(1_u64)]))),
        );
        let adapter = MethodInvocationRetryListenerSupport::new(CountingListener::default());

        assert!(adapter.open(&context));
        adapter.on_error(
            &context,
            &crate::error::retry_error::RetryError::custom("boom"),
        );

        assert_eq!(adapter.listener().opens.load(Ordering::Relaxed), 1);
        assert_eq!(adapter.listener().errors.load(Ordering::Relaxed), 1);
        assert!(Arc::strong_count(&Arc::new(())) >= 1);
    }

    #[test]
    fn a_plain_retry_is_ignored() {
        let context = crate::context::retry_context_support::RetryContextSupport::new();
        let adapter = MethodInvocationRetryListenerSupport::new(CountingListener::default());

        assert!(adapter.open(&context));
        adapter.on_error(
            &context,
            &crate::error::retry_error::RetryError::custom("boom"),
        );

        assert_eq!(adapter.listener().opens.load(Ordering::Relaxed), 0);
        assert_eq!(adapter.listener().errors.load(Ordering::Relaxed), 0);
    }
}
