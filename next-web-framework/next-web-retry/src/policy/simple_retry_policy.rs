use std::{any::Any, collections::HashMap, sync::Arc};

use crate::{
    classifier::{binary_Error_classifier::BinaryExceptionClassifier, classifier::Classifier},
    error::{CloneableError, retry_error::RetryError},
    retry_context::RetryContext,
    retry_policy::RetryPolicy,
};

#[derive(Clone)]
pub struct SimpleRetryPolicy {
    max_attempts: i32,
    retryable_classifier: BinaryExceptionClassifier,
    recoverable_classifier: BinaryExceptionClassifier,
    max_attempts_supplier: Option<Arc<dyn Fn() -> i32 + Send + Sync>>,
}

impl SimpleRetryPolicy {
   

    pub fn new(
        max_attempts: i32,
        retryable_errors: HashMap<RetryError, bool>,
        traverse_causes: bool,
        default_value: bool,
    ) -> Self {
        let mut retryable_classifier =
            BinaryExceptionClassifier::with_retryable_errors_and_default_value(
                retryable_errors,
                default_value,
            );
        retryable_classifier.set_traverse_causes(traverse_causes);
        Self {
            max_attempts,
            retryable_classifier,
            recoverable_classifier: BinaryExceptionClassifier::new(
                HashMap::with_capacity(1),
                true,
                true,
            ),
            max_attempts_supplier: None,
        }
    }

    pub fn with_max_attempts(max_attempts: i32) -> Self {
        Self::with_binary_classifier(
            max_attempts,
            BinaryExceptionClassifier::default_classifier(),
        )
    }

    pub fn with_retryable_errors(
        max_attempts: i32,
        retryable_errors: HashMap<RetryError, bool>,
    ) -> Self {
        Self::with_retryable_errors_and_traverse_causes(max_attempts, retryable_errors, false)
    }

    pub fn with_retryable_errors_and_traverse_causes(
        max_attempts: i32,
        retryable_errors: HashMap<RetryError, bool>,
        traverse_causes: bool,
    ) -> Self {
        Self::new(max_attempts, retryable_errors, traverse_causes, false)
    }

    pub fn with_binary_classifier(
        max_attempts: i32,
        classifier: BinaryExceptionClassifier,
    ) -> Self {
        Self {
            max_attempts,
            retryable_classifier: classifier,
            recoverable_classifier: BinaryExceptionClassifier::new(Default::default(), true, true),
            max_attempts_supplier: None,
        }
    }
}


const DEFAULT_MAX_ATTEMPTS: i32 = 3;


impl SimpleRetryPolicy {
    pub fn set_max_attempts(&mut self, max_attempts: i32) {
        self.max_attempts = max_attempts;
    }

    pub fn set_not_recoverable(&mut self, no_recovery: Vec<RetryError>) {
        let type_map = no_recovery
            .into_iter()
            .map(|s| (s, false))
            .collect::<HashMap<_, _>>();
        self.recoverable_classifier = BinaryExceptionClassifier::new(type_map, true, true)
    }

    pub fn set_max_attempts_supplier<F>(&mut self, max_attempts_supplier: F)
    where
        F: Fn() -> i32 + Send + Sync + 'static,
    {
        self.max_attempts_supplier = Some(Arc::new(max_attempts_supplier));
    }

    fn retry_for_error(&self, error: &Option<Box<dyn CloneableError>>) -> bool {
        match error {
            Some(e) => self.recoverable_classifier.classify(e),
            None => false,
        }
    }
}

impl Default for SimpleRetryPolicy {
    fn default() -> Self {
        Self::with_max_attempts(DEFAULT_MAX_ATTEMPTS)
    }
}

impl RetryPolicy for SimpleRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {
        let error = context.get_last_error();
        if (error.is_none() || self.retry_for_error(&error))
            && (context.get_retry_count() as i32) < self.get_max_attempts()
        {
            false
        } else {
            true
        }
    }

    // fn open(&self, parent: impl RetryContext) -> Option<Arc<dyn RetryContext>> {
    //     return Some(Arc::new(SimpleRetryContext::new(parent)));
    // }

    fn close(&self, _context: &dyn RetryContext) {}

    fn register_error(
        &mut self,
        context: &mut dyn RetryContext,
        error: Option<&dyn CloneableError>,
    ) {
        let ctx: &mut dyn Any = context;
        match ctx.downcast_mut::<SimpleRetryContext>() {
            Some(simple_context) => {
                simple_context.register_error(error.map(|s| s.to_boxed()));
            }
            None => {}
        }
    }

    fn get_max_attempts(&self) -> i32 {
        if let Some(max_attempts_supplier) = self.max_attempts_supplier.as_ref() {
            return max_attempts_supplier();
        }
        return self.max_attempts;
    }
}

struct SimpleRetryContext {
    parent: Option<Arc<dyn RetryContext>>,
    count: u16,
    last_error: Option<Box<dyn CloneableError>>,
    terminate: bool,
}

impl SimpleRetryContext {
    pub fn new<C>(context: C) -> Self
    where
        C: RetryContext + 'static,
    {
        Self {
            parent: Some(Arc::new(context)),
            count: 0,
            last_error: None,
            terminate: false,
        }
    }

    pub fn register_error(&mut self, error: Option<Box<dyn CloneableError>>) {
        if let Some(error) = error {
            self.last_error = Some(error);
            self.count += 1;
        }
    }
}

impl RetryContext for SimpleRetryContext {
    fn set_exhausted_only(&mut self) {
        self.terminate = true;
    }

    fn is_exhausted_only(&self) -> bool {
        self.terminate
    }

    fn get_parent(&self) -> Option<&dyn RetryContext> {
        self.parent.as_ref().map(|p| p.as_ref())
    }

    fn get_retry_count(&self) -> u16 {
        self.count
    }

    fn get_last_error(&self) -> Option<Box<dyn CloneableError>> {
        self.last_error.as_ref().cloned()
    }
}
