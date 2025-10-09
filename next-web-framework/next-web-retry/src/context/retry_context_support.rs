use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU16, Ordering},
};

use next_web_core::models::{any_error::AnyError, any_value::AnyValue};
use tokio::sync::Mutex;

use crate::{
    error::retry_error::RetryError,
    retry_context::{AttributeAccessorSupport, RetryContext, SyncAttributeAccessor},
};

#[derive(Clone)]
pub struct RetryContextSupport {
    parent: Option<Arc<dyn RetryContext>>,
    count: Arc<AtomicU16>,
    terminate: Arc<AtomicBool>,
    last_error: Arc<Mutex<Option<RetryError>>>,
    attribute_support: AttributeAccessorSupport,
}

impl RetryContextSupport {
    pub fn with_parent(parent: Option<Arc<dyn RetryContext>>) -> Self {
        let mut s = Self::default();
        s.parent = parent;
        s
    }

    pub fn register_error(&self, error: Option<&dyn AnyError>) {
        if let Some(error) = error {
            self.count.fetch_add(1, Ordering::Relaxed);
            self.last_error
                .try_lock()
                .map(|mut lock| lock.replace(RetryError::Any(error.to_boxed())))
                .ok();
        }
    }
}

impl SyncAttributeAccessor for RetryContextSupport {
    fn has_attribute(&self, name: &str) -> bool {
        self.attribute_support.has_attribute(name)
    }

    fn set_attribute(&self, name: &str, value: AnyValue) {
        self.attribute_support.set_attribute(name, value)
    }

    fn remove_attribute(&self, name: &str) -> Option<AnyValue> {
        self.attribute_support.remove_attribute(name)
    }

    fn get_attribute(&self, name: &str) -> Option<AnyValue> {
        self.attribute_support.get_attribute(name)
    }
}

impl RetryContext for RetryContextSupport {
    fn set_exhausted_only(&self) {
        self.terminate.store(true, Ordering::Relaxed)
    }

    fn is_exhausted_only(&self) -> bool {
        self.terminate.load(Ordering::Relaxed)
    }

    fn get_parent(&self) -> Option<&dyn RetryContext> {
        self.parent.as_deref()
    }

    fn get_retry_count(&self) -> u16 {
        self.count.load(Ordering::Relaxed)
    }

    fn get_last_error(&self) -> Option<RetryError> {
        self.last_error.try_lock().map(|lock| lock.clone()).unwrap_or_default()
    }
}

impl Default for RetryContextSupport {
    fn default() -> Self {
        Self {
            parent: None,
            count: Arc::new(AtomicU16::new(0)),
            terminate: Arc::new(AtomicBool::new(false)),
            last_error: Arc::new(Mutex::new(None)),
            attribute_support: AttributeAccessorSupport::default(),
        }
    }
}
