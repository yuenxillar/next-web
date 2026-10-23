//! Retry policies.
//!
//! A policy decides whether a retry block may make another attempt, and owns the per-block
//! state through the retry context it creates. The policies here compose: a counting policy
//! bounds the attempts, a classifier policy decides which errors deserve them, and
//! [`composite_retry_policy::CompositeRetryPolicy`] combines both.

pub mod abstract_map_retry_context_cache;
pub mod always_retry_policy;
pub mod binary_error_classifier_retry_policy;
pub mod circuit_breaker_retry_policy;
pub mod composite_retry_policy;
pub mod exception_classifier_retry_policy;
pub mod expression_retry_policy;
pub mod map_retry_context_cache;
pub mod max_attempts_retry_policy;
pub mod never_retry_policy;
pub mod predicate_retry_policy;
pub mod retry_context_cache;
pub mod simple_retry_policy;
pub mod soft_reference_map_retry_context_cache;
pub mod timeout_retry_policy;

pub use abstract_map_retry_context_cache::AbstractMapRetryContextCache;
pub use always_retry_policy::AlwaysRetryPolicy;
pub use binary_error_classifier_retry_policy::BinaryErrorClassifierRetryPolicy;
pub use circuit_breaker_retry_policy::CircuitBreakerRetryPolicy;
pub use composite_retry_policy::CompositeRetryPolicy;
pub use exception_classifier_retry_policy::ExceptionClassifierRetryPolicy;
pub use expression_retry_policy::{Expression, ExpressionRetryPolicy, LiteralExpression};
pub use map_retry_context_cache::MapRetryContextCache;
pub use max_attempts_retry_policy::MaxAttemptsRetryPolicy;
pub use never_retry_policy::NeverRetryPolicy;
pub use predicate_retry_policy::PredicateRetryPolicy;
pub use retry_context_cache::RetryContextCache;
pub use simple_retry_policy::SimpleRetryPolicy;
pub use soft_reference_map_retry_context_cache::SoftReferenceMapRetryContextCache;
pub use timeout_retry_policy::TimeoutRetryPolicy;
