use crate::{classifier::binary_error_classifier::BinaryErrorClassifier, retry_policy::RetryPolicy};



#[derive(Clone)]
pub struct BinaryErrorClassifierRetryPolicy {

}


impl BinaryErrorClassifierRetryPolicy {

    pub fn new(error_classifier: BinaryErrorClassifier ) -> Self {
        Self {}
    }
}

impl RetryPolicy for BinaryErrorClassifierRetryPolicy {
    fn can_retry(&self, context: &dyn crate::retry_context::RetryContext) -> bool {
        todo!()
    }

    fn open(&self, context:  Option<&dyn crate::retry_context::RetryContext>) -> Box<dyn crate::retry_context::RetryContext> {
        todo!()
    }

    fn close(&self, context: &dyn crate::retry_context::RetryContext) {
        todo!()
    }

    fn register_error(&self, context: &mut dyn crate::retry_context::RetryContext, error: Option<&dyn crate::error::AnyError>) {
        todo!()
    }
}