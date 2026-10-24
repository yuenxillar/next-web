//! Interceptors that retry a call without the caller writing a retry loop.
//!
//! An interceptor wraps one call. The call is described by a
//! [`method_invocation::MethodInvocation`] and performed by a closure, which replaces the
//! reflective method invocation of the original API. Three interceptors are provided:
//!
//! * [`retry_operations_interceptor::RetryOperationsInterceptor`] retries in place until the
//!   call succeeds or the policy gives up;
//! * [`stateful_retry_operations_interceptor::StatefulRetryOperationsInterceptor`] rethrows and
//!   remembers the item, so the next invocation resumes the retry;
//! * both are built with [`retry_interceptor_builder::RetryInterceptorBuilder`].

pub mod fixed_key_generator;
pub mod method_arguments_key_generator;
pub mod method_invocation;
pub mod method_invocation_recoverer;
pub mod method_invocation_retry_callback;
pub mod new_method_arguments_identifier;
pub mod retry_interceptor_builder;
pub mod retry_operations_interceptor;
pub mod retryable;
pub mod stateful_retry_operations_interceptor;

pub use fixed_key_generator::FixedKeyGenerator;
pub use method_arguments_key_generator::MethodArgumentsKeyGenerator;
pub use method_invocation::MethodInvocation;
pub use method_invocation_recoverer::{ConstantRecoverer, MethodInvocationRecoverer};
pub use method_invocation_retry_callback::{
    ARGS, METHOD, METHOD_ARGS, MethodInvocationRetryCallback,
};
pub use new_method_arguments_identifier::NewMethodArgumentsIdentifier;
pub use retry_interceptor_builder::{
    CircuitBreakerInterceptorBuilder, RetryInterceptorBuilder, StatefulRetryInterceptorBuilder,
    StatelessRetryInterceptorBuilder,
};
pub use retry_operations_interceptor::RetryOperationsInterceptor;
pub use retryable::Retryable;
pub use stateful_retry_operations_interceptor::StatefulRetryOperationsInterceptor;
