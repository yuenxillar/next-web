//! Retry listener base types.
//!
//! [`retry_listener_support::RetryListenerSupport`] is a listener that implements nothing, and
//! [`method_invocation_retry_listener_support::MethodInvocationRetryListenerSupport`] adapts a
//! listener that wants the intercepted call description into a plain retry listener.

pub mod method_invocation_retry_listener_support;
pub mod retry_listener_support;

pub use method_invocation_retry_listener_support::{
    MethodInvocationRetryListener, MethodInvocationRetryListenerSupport,
};
pub use retry_listener_support::RetryListenerSupport;
