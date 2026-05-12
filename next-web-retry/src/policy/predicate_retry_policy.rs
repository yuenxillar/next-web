use std::{any::Any, sync::Arc};

use next_web_core::{anys::any_error::AnyError, async_trait, traits::named::Named};

use crate::{
    Predicate, context::retry_context_support::RetryContextSupport, error::retry_error::RetryError,
    retry_policy::RetryPolicy,
};

#[derive(Clone)]
pub struct PredicateRetryPolicy {
    predicate: Arc<dyn Predicate<RetryError>>,
}

impl PredicateRetryPolicy {
    pub fn new(predicate: Arc<dyn Predicate<RetryError>>) -> Self {
        Self { predicate }
    }
}

#[async_trait]
impl RetryPolicy for PredicateRetryPolicy {
    async fn can_retry(&self, context: &dyn crate::retry_context::RetryContext) -> bool {
        match context.get_last_error() {
            Some(error) => self.predicate.test(&error),
            None => true,
        }
    }

    fn open(
        &self,
        _context: Option<&dyn crate::retry_context::RetryContext>,
    ) -> Arc<dyn crate::retry_context::RetryContext> {
        Arc::new(RetryContextSupport::default())
    }

    fn close(&self, _context: &dyn crate::retry_context::RetryContext) {}

    fn register_error(
        &self,
        context: &dyn crate::retry_context::RetryContext,
        error: Option<&dyn AnyError>,
    ) {
        let any: &dyn Any = context;
        if let Some(context) = any.downcast_ref::<RetryContextSupport>() {
            context.register_error(error);
        }
    }
}

impl Named for PredicateRetryPolicy {
    fn name(&self) -> &str {
        "PredicateRetryPolicy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn predicate_retry_policy_allows_first_attempt() {
        let policy = PredicateRetryPolicy::new(Arc::new(|_: &RetryError| false));
        let context = policy.open(None);

        assert!(policy.can_retry(context.as_ref()).await);
    }

    #[tokio::test]
    async fn predicate_retry_policy_uses_last_error() {
        let policy = PredicateRetryPolicy::new(Arc::new(
            |error: &RetryError| matches!(error, RetryError::Custom(message) if message == "failed"),
        ));
        let context = policy.open(None);
        let error = RetryError::Custom("failed".to_string())
            .as_any_error()
            .expect("custom retry error should convert to AnyError");

        policy.register_error(context.as_ref(), Some(error.as_ref()));

        assert_eq!(context.get_retry_count(), 1);
        assert!(policy.can_retry(context.as_ref()).await);
    }

    #[tokio::test]
    async fn predicate_retry_policy_can_reject_last_error() {
        let policy = PredicateRetryPolicy::new(Arc::new(|_: &RetryError| false));
        let context = policy.open(None);
        let error = RetryError::Custom("failed".to_string())
            .as_any_error()
            .expect("custom retry error should convert to AnyError");

        policy.register_error(context.as_ref(), Some(error.as_ref()));

        assert!(!policy.can_retry(context.as_ref()).await);
    }
}
