//! A policy that retries only while an expression accepts the context.

use std::sync::Arc;

use next_web_core::Named;

use crate::{
    error::retry_error::RetryError, retry_context::RetryContext, retry_policy::RetryPolicy,
};

use super::simple_retry_policy::SimpleRetryPolicy;

/// A condition evaluated against a retry context.
///
/// This is the Rust counterpart of an expression: instead of parsing a string into an
/// expression tree, a policy is given a value that answers the question directly. A closure
/// over [`RetryError`] is the usual implementation, and it can inspect anything the context
/// exposes, including the message, the error kind and the attempt count.
///
/// # Example
///
/// ```
/// use std::sync::Arc;
///
/// use next_web_retry::{
///     error::retry_error::RetryError,
///     policy::expression_retry_policy::ExpressionRetryPolicy,
///     retry_policy::RetryPolicy,
/// };
///
/// let policy = ExpressionRetryPolicy::new(Arc::new(|context: &dyn next_web_retry::retry_context::RetryContext| {
///     context
///         .get_last_error()
///         .map(|error| error.message().contains("timeout"))
///         .unwrap_or(true)
/// }));
/// # let _ = policy;
/// ```
pub trait Expression
where
    Self: Send + Sync,
{
    /// Returns whether the retry may continue.
    fn evaluate(&self, context: &dyn RetryContext) -> bool;
}

impl<F> Expression for F
where
    F: Fn(&dyn RetryContext) -> bool + Send + Sync,
{
    fn evaluate(&self, context: &dyn RetryContext) -> bool {
        self(context)
    }
}

/// An expression that always returns the same answer.
#[derive(Debug, Clone, Copy)]
pub struct LiteralExpression {
    value: bool,
}

impl LiteralExpression {
    /// Creates an expression that always returns `value`.
    pub fn new(value: bool) -> Self {
        Self { value }
    }
}

impl Expression for LiteralExpression {
    fn evaluate(&self, _context: &dyn RetryContext) -> bool {
        self.value
    }
}

/// A policy that delegates to a [`SimpleRetryPolicy`] and additionally evaluates an
/// [`Expression`].
///
/// It answers `true` only when the delegate allows the retry *and* the expression accepts
/// the context. Before the first failure there is nothing to evaluate, so the delegate
/// decides alone. This lets a retry continue while, for example, the error message still
/// looks transient, even when the attempt budget allows more tries.
#[derive(Clone)]
pub struct ExpressionRetryPolicy {
    policy: SimpleRetryPolicy,
    expression: Arc<dyn Expression>,
}

impl ExpressionRetryPolicy {
    /// Creates a policy with the default attempt limit and the given expression.
    pub fn new(expression: Arc<dyn Expression>) -> Self {
        Self {
            policy: SimpleRetryPolicy::default(),
            expression,
        }
    }

    /// Creates a policy from an existing delegate and an expression.
    pub fn with_policy(policy: SimpleRetryPolicy, expression: Arc<dyn Expression>) -> Self {
        Self { policy, expression }
    }

    /// Returns the delegate policy.
    pub fn policy(&self) -> &SimpleRetryPolicy {
        &self.policy
    }
}

impl RetryPolicy for ExpressionRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {
        if context.get_last_error().is_none() {
            return self.policy.can_retry(context);
        }

        self.policy.can_retry(context) && self.expression.evaluate(context)
    }

    fn open(&self, parent: Option<Arc<dyn RetryContext>>) -> Arc<dyn RetryContext> {
        self.policy.open(parent)
    }

    fn close(&self, context: &dyn RetryContext) {
        self.policy.close(context);
    }

    fn register_error(&self, context: &dyn RetryContext, error: &RetryError) {
        self.policy.register_error(context, error);
    }

    fn get_max_attempts(&self) -> u16 {
        self.policy.get_max_attempts()
    }
}

impl Named for ExpressionRetryPolicy {
    fn name(&self) -> &str {
        "ExpressionRetryPolicy"
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{ExpressionRetryPolicy, LiteralExpression};
    use crate::{error::retry_error::RetryError, retry_policy::RetryPolicy};

    #[test]
    fn the_delegate_decides_before_the_first_failure() {
        let policy = ExpressionRetryPolicy::with_policy(
            crate::policy::simple_retry_policy::SimpleRetryPolicy::with_max_attempts(2),
            Arc::new(LiteralExpression::new(false)),
        );
        let context = policy.open(None);

        assert!(policy.can_retry(context.as_ref()));
    }

    #[test]
    fn the_expression_can_stop_a_retry_the_delegate_allows() {
        let policy = ExpressionRetryPolicy::with_policy(
            crate::policy::simple_retry_policy::SimpleRetryPolicy::with_max_attempts(5),
            Arc::new(|context: &dyn crate::retry_context::RetryContext| {
                context
                    .get_last_error()
                    .map(|error| error.message() == "transient")
                    .unwrap_or(true)
            }),
        );
        let context = policy.open(None);

        policy.register_error(context.as_ref(), &RetryError::custom("transient"));
        assert!(policy.can_retry(context.as_ref()));

        policy.register_error(context.as_ref(), &RetryError::custom("fatal"));
        assert!(!policy.can_retry(context.as_ref()));
    }
}
