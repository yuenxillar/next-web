use std::sync::Arc;

use next_web_core::{async_trait, models::any_error::AnyError};

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

#[async_trait]
impl RetryPolicy for PredicateRetryPolicy {
    async fn can_retry(&self, context: &dyn crate::retry_context::RetryContext) -> bool {
        todo!()
    }

    fn open(
        &self,
        context: Option<&dyn crate::retry_context::RetryContext>,
    ) -> Arc<dyn crate::retry_context::RetryContext> {
        todo!()
    }

    fn close(&self, _context: &dyn crate::retry_context::RetryContext) {
        
    }

    fn register_error(
        &self,
        context: &dyn crate::retry_context::RetryContext,
        error: Option<&dyn AnyError>,
    ) {
        todo!()
    }
}

impl ToString for PredicateRetryPolicy {
    fn to_string(&self) -> String {
        "PredicateRetryPolicy".to_string()
    }
}