use std::sync::Arc;

use crate::retry_policy::RetryPolicy;

#[derive(Clone)]
pub struct CompositeRetryPolicy {}

impl CompositeRetryPolicy {
    pub fn new() -> Self {
        Self {}
    }

    pub fn set_policies(&mut self, policies: Vec<Arc<dyn RetryPolicy>>) {}
}

impl RetryPolicy for CompositeRetryPolicy {
    fn can_retry(&self, context: &dyn crate::retry_context::RetryContext) -> bool {
        todo!()
    }

    fn open(
        &self,
        context: Option<&dyn crate::retry_context::RetryContext>,
    ) -> Box<dyn crate::retry_context::RetryContext> {
        todo!()
    }

    fn close(&self, context: &dyn crate::retry_context::RetryContext) {
        todo!()
    }

    fn register_error(
        &self,
        context: &mut dyn crate::retry_context::RetryContext,
        error: Option<&dyn crate::error::AnyError>,
    ) {
        todo!()
    }
}
