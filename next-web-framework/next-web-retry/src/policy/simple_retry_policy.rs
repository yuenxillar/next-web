use std::{any::Any, collections::HashMap, sync::{atomic::{AtomicBool, AtomicU16, Ordering}, Arc}};

use next_web_core::{async_trait, anys::{any_error::AnyError, any_value::AnyValue}};

use crate::{
    classifier::{binary_error_classifier::BinaryErrorClassifier, classifier::Classifier}, error::retry_error::RetryError, retry_context::{RetryContext, SyncAttributeAccessor}, retry_policy::RetryPolicy
};

#[derive(Clone)]
pub struct SimpleRetryPolicy {
    max_attempts: u16,
    retryable_classifier: BinaryErrorClassifier,
    recoverable_classifier: BinaryErrorClassifier,
    max_attempts_supplier: Option<Arc<dyn Fn() -> u16 + Send + Sync>>,
}

impl SimpleRetryPolicy {
    pub fn new(
        max_attempts: u16,
        retryable_errors: HashMap<RetryError, bool>,
        traverse_causes: bool,
        default_value: bool,
    ) -> Self {
        let mut retryable_classifier =
        BinaryErrorClassifier::with_retryable_errors_and_default_value(
                retryable_errors,
                default_value,
            );
        retryable_classifier.set_traverse_causes(traverse_causes);
        Self {
            max_attempts,
            retryable_classifier,
            recoverable_classifier: BinaryErrorClassifier::new(
                HashMap::with_capacity(1),
                true,
                true,
            ),
            max_attempts_supplier: None,
        }
    }

    pub fn with_max_attempts(max_attempts: u16) -> Self {
        Self::with_binary_classifier(
            max_attempts,
            BinaryErrorClassifier::default_classifier(),
        )
    }

    pub fn with_retryable_errors(
        max_attempts: u16,
        retryable_errors: HashMap<RetryError, bool>,
    ) -> Self {
        Self::with_retryable_errors_and_traverse_causes(max_attempts, retryable_errors, false)
    }

    pub fn with_retryable_errors_and_traverse_causes(
        max_attempts: u16,
        retryable_errors: HashMap<RetryError, bool>,
        traverse_causes: bool,
    ) -> Self {
        Self::new(max_attempts, retryable_errors, traverse_causes, false)
    }

    pub fn with_binary_classifier(
        max_attempts: u16,
        classifier: BinaryErrorClassifier,
    ) -> Self {
        Self {
            max_attempts,
            retryable_classifier: classifier,
            recoverable_classifier: BinaryErrorClassifier::new(Default::default(), true, true),
            max_attempts_supplier: None,
        }
    }
}

const DEFAULT_MAX_ATTEMPTS: u16 = 3;

impl SimpleRetryPolicy {
    pub fn set_max_attempts(&mut self, max_attempts: u16) {
        self.max_attempts = max_attempts;
    }

    pub fn set_not_recoverable(&mut self, no_recovery: Vec<RetryError>) {
        let type_map = no_recovery
            .into_iter()
            .map(|s| (s, false))
            .collect::<HashMap<_, _>>();
        self.recoverable_classifier = BinaryErrorClassifier::new(type_map, true, true)
    }

    pub fn set_max_attempts_supplier<F>(&mut self, max_attempts_supplier: F)
    where
        F: Fn() -> u16 + Send + Sync + 'static,
    {
        self.max_attempts_supplier = Some(Arc::new(max_attempts_supplier));
    }

    async fn retry_for_error(&self, error: Option<&RetryError>) -> bool {
        self.recoverable_classifier.classify(error).await
    }
}

impl Default for SimpleRetryPolicy {
    fn default() -> Self {
        Self::with_max_attempts(DEFAULT_MAX_ATTEMPTS)
    }
}

#[async_trait]
impl RetryPolicy for SimpleRetryPolicy {
    async fn can_retry(&self, context: &dyn RetryContext) -> bool {
        let error = context.get_last_error();
        if (error.is_none() || self.retry_for_error(error.as_ref()).await)
            && context.get_retry_count() < self.get_max_attempts()
        {
            false
        } else {
            true
        }
    }

    fn open(&self, parent: Option<&dyn RetryContext>) -> Arc<dyn RetryContext> {
        return Arc::new(SimpleRetryContext::default());
    }

    fn close(&self, _context: &dyn RetryContext) {}

    fn register_error(&self, context: &dyn RetryContext, error: Option<&dyn AnyError>) {
        let ctx: & dyn Any = context;
        match ctx.downcast_ref::<SimpleRetryContext>() {
            Some(simple_context) => {
                simple_context.register_error(error.map(|s| s.to_boxed()));
            }
            None => {}
        }
    }

    fn get_max_attempts(&self) -> u16 {
        if let Some(max_attempts_supplier) = self.max_attempts_supplier.as_ref() {
            return max_attempts_supplier();
        }
        return self.max_attempts;
    }
}

#[derive(Clone)]
struct SimpleRetryContext {
    pub(crate) parent: Option<Arc<dyn RetryContext>>,
    count: Arc<AtomicU16>,
    last_error: Option<RetryError>,
    terminate: Arc<AtomicBool>,
}

impl SimpleRetryContext {
    pub fn new<C>(context: C) -> Self
    where
        C: RetryContext + 'static,
    {
        Self {
            parent: Some(Arc::new(context)),
            count: Arc::new(AtomicU16::new(0)),
            last_error: None,
            terminate: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn register_error(&self, error: Option<Box<dyn AnyError>>) {
        if let Some(error) = error {
            // TODO
            // self.last_error.replace(Some(RetryError::Any(error)));
            self.count.store(self.count.load(Ordering::Relaxed), Ordering::Relaxed);
        }
    }
}

impl Default for SimpleRetryContext {
    fn default() -> Self {
        Self {
            parent: None,
            count: Arc::new(AtomicU16::new(0)),
            last_error: None,
            terminate: Arc::new(AtomicBool::new(false)),
        }
    }
}


impl SyncAttributeAccessor for SimpleRetryContext { 
    fn has_attribute(&self, name: &str) -> bool {
        todo!()
    }

    fn set_attribute(&self, name: &str, value: AnyValue) {
        todo!()
    }

    fn remove_attribute(&self, name: &str) -> Option<AnyValue> {
        todo!()
    }

    fn get_attribute(&self, name: &str) -> Option<AnyValue> {
        todo!()
    }
}

impl RetryContext for SimpleRetryContext {
    fn set_exhausted_only(&self) {
        self.terminate.store(true, Ordering::Relaxed);
    }

    fn is_exhausted_only(&self) -> bool {
        self.terminate.load(Ordering::Relaxed)
    }

    fn get_parent(&self) -> Option<&dyn RetryContext> {
        self.parent.as_ref().map(|p| p.as_ref())
    }

    fn get_retry_count(&self) -> u16 {
        self.count.load(Ordering::Relaxed)
    }

    fn get_last_error(&self) -> Option<RetryError> {
        self.last_error.clone()
    }
}


impl ToString for SimpleRetryPolicy {
    fn to_string(&self) -> String {
        "SimpleRetryPolicy".to_string()
    }
}