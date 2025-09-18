use std::sync::Arc;

use crate::retry_policy::RetryPolicy;



#[derive(Clone, Default)]
pub struct AlwaysRetryPolicy;


impl RetryPolicy for AlwaysRetryPolicy {
    fn can_retry(&self, context: &dyn crate::retry_context::RetryContext) -> bool {
        true
    }

    fn open(
        &self,
        context: Option<&dyn crate::retry_context::RetryContext>,
    ) -> Arc<dyn crate::retry_context::RetryContext> {
        todo!()
    }

    fn close(&self, context: &dyn crate::retry_context::RetryContext) {
        todo!()
    }

    fn register_error(
        &self,
        context: &dyn crate::retry_context::RetryContext,
        error: Option<&dyn crate::error::AnyError>,
    ) {
        todo!()
    }
}
