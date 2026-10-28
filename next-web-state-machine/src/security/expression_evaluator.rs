//! Evaluation of security expressions.
//!
//! Spring Security evaluates SpEL expressions against an expression root. Rust
//! has no SpEL equivalent, so an expression rule is evaluated by a pluggable
//! [`ExpressionEvaluator`](crate::security::expression_evaluator::ExpressionEvaluator).
//! Without one an expression rule denies the operation, which fails closed
//! instead of silently allowing it.

use next_web_core::any::AnyValue;

/// Evaluates a security expression.
pub trait ExpressionEvaluator: Send + Sync {
    /// Evaluates the expression for the given authentication.
    ///
    /// # Arguments
    /// * `expression` - the expression of the security rule
    /// * `authentication` - the authentication of the caller
    ///
    /// # Returns
    /// `true` when the expression grants access
    fn evaluate(
        &self,
        expression: &str,
        authentication: &AnyValue,
    ) -> Result<bool, next_web_core::error::BoxError>;
}
