use std::{any::Any, sync::Arc};

use next_web_core::{anys::any_error::AnyError, async_trait, traits::named::Named};

use crate::{context::retry_context_support::RetryContextSupport, retry_policy::RetryPolicy};

#[derive(Clone, Default)]
pub struct AlwaysRetryPolicy;

#[async_trait]
impl RetryPolicy for AlwaysRetryPolicy {
    async fn can_retry(&self, _context: &dyn crate::retry_context::RetryContext) -> bool {
        true
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

impl Named for AlwaysRetryPolicy {
    fn name(&self) -> &str {
        "AlwaysRetryPolicy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::retry_error::RetryError;

    #[tokio::test]
    async fn always_retry_policy_records_errors_and_keeps_retrying() {
        let policy = AlwaysRetryPolicy;
        let context = policy.open(None);
        let error = RetryError::Custom("failed".to_string())
            .as_any_error()
            .expect("custom retry error should convert to AnyError");

        assert!(policy.can_retry(context.as_ref()).await);

        policy.register_error(context.as_ref(), Some(error.as_ref()));

        assert_eq!(context.get_retry_count(), 1);
        assert!(context.get_last_error().is_some());
        assert!(policy.can_retry(context.as_ref()).await);
    }
}
