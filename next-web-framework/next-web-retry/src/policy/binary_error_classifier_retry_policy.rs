use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    classifier::{binary_error_classifier::BinaryErrorClassifier, classifier::Classifier},
    context::retry_context_support::RetryContextSupport,
    error::AnyError,
    retry_context::RetryContext,
    retry_policy::RetryPolicy,
};

#[derive(Clone)]
pub struct BinaryErrorClassifierRetryPolicy {
    error_classifier: BinaryErrorClassifier,
}

impl BinaryErrorClassifierRetryPolicy {
    pub fn new(error_classifier: BinaryErrorClassifier) -> Self {
        Self { error_classifier }
    }
}


#[async_trait]
impl RetryPolicy for BinaryErrorClassifierRetryPolicy {
    async fn can_retry(&self, context: &dyn RetryContext) -> bool {
        let last_error = context.get_last_error();
        last_error.is_none() || self.error_classifier.classify(last_error.as_ref()).await
    }

    fn open(&self, context: Option<&dyn RetryContext>) -> Arc<dyn RetryContext> {
        // TODO parent
        Arc::new(RetryContextSupport::with_parent(None))
    }

    fn close(&self, _context: &dyn RetryContext) {}

    fn register_error(&self, context: &dyn RetryContext, error: Option<&dyn AnyError>) {
        let any: & dyn std::any::Any = context;
        if let Some(support) = any.downcast_ref::<RetryContextSupport>() {
            support.register_error(error);
        }
    }
}

impl ToString for BinaryErrorClassifierRetryPolicy {
    fn to_string(&self) -> String {
        "BinaryErrorClassifierRetryPolicy".to_string()
    }
}