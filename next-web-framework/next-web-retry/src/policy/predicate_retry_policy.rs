use std::sync::Arc;

use crate::{error::retry_error::RetryError, retry_policy::RetryPolicy, Predicate};

#[derive(Clone)]
pub struct PredicateRetryPolicy {
    predicate: Arc<dyn Predicate<RetryError>>
}

impl PredicateRetryPolicy {
    pub fn new(
        predicate: Arc<dyn Predicate<RetryError>>,
    ) -> Self {
        Self {
            predicate,
        }
    }
}

impl RetryPolicy for PredicateRetryPolicy {
    fn can_retry(&self, context: &dyn crate::retry_context::RetryContext) -> bool {
        todo!()
    }

    fn open(
        &self,
        context: Option<&dyn crate::retry_context::RetryContext>,
    ) -> Box<dyn crate::retry_context::RetryContext> {
        todo!()
    }

    fn close(&self, _context: &dyn crate::retry_context::RetryContext) {
        
    }

    fn register_error(
        &self,
        context: &mut dyn crate::retry_context::RetryContext,
        error: Option<&dyn crate::error::AnyError>,
    ) {
        todo!()
    }
}
