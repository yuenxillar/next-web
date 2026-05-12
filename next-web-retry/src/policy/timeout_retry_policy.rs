use std::{any::Any, sync::Arc};

use next_web_core::{
    anys::{any_error::AnyError, any_value::AnyValue},
    async_trait,
    traits::named::Named,
};

use crate::{
    context::retry_context_support::RetryContextSupport,
    error::retry_error::RetryError,
    retry_context::{RetryContext, SyncAttributeAccessor},
    retry_policy::RetryPolicy,
};

#[derive(Clone)]
pub struct TimeoutRetryPolicy {
    timeout: u64,
}

impl TimeoutRetryPolicy {
    pub fn new(timeout: u64) -> Self {
        Self { timeout }
    }

    pub fn timeout(&self) -> u64 {
        self.timeout
    }

    pub fn set_timeout(&mut self, timeout: u64) {
        self.timeout = timeout;
    }
}

impl Default for TimeoutRetryPolicy {
    fn default() -> Self {
        Self { timeout: 1000 }
    }
}

#[async_trait]
impl RetryPolicy for TimeoutRetryPolicy {
    async fn can_retry(&self, context: &dyn RetryContext) -> bool {
        let any: &dyn Any = context;
        match any.downcast_ref::<TimeoutRetryContext>() {
            Some(context) => context.is_alive(),
            None => false,
        }
    }

    fn open(&self, _context: Option<&dyn RetryContext>) -> Arc<dyn RetryContext> {
        Arc::new(TimeoutRetryContext::new(self.timeout))
    }

    fn close(&self, _context: &dyn RetryContext) {}

    fn register_error(&self, context: &dyn RetryContext, error: Option<&dyn AnyError>) {
        let context: &dyn Any = context;
        if let Some(context) = context.downcast_ref::<TimeoutRetryContext>() {
            context.context_support.register_error(error);
        }
    }
}

#[derive(Clone)]
struct TimeoutRetryContext {
    timeout: u64,
    start: u64,
    context_support: RetryContextSupport,
}

impl TimeoutRetryContext {
    pub fn new(timeout: u64) -> Self {
        Self {
            start: timestamp(),
            timeout,
            context_support: RetryContextSupport::default(),
        }
    }

    fn is_alive(&self) -> bool {
        (timestamp() - self.start) <= self.timeout
    }
}

impl SyncAttributeAccessor for TimeoutRetryContext {
    fn has_attribute(&self, name: &str) -> bool {
        self.context_support.has_attribute(name)
    }

    fn set_attribute(&self, name: &str, value: AnyValue) {
        self.context_support.set_attribute(name, value)
    }

    fn remove_attribute(&self, name: &str) -> Option<AnyValue> {
        self.context_support.remove_attribute(name)
    }

    fn get_attribute(&self, name: &str) -> Option<AnyValue> {
        self.context_support.get_attribute(name)
    }
}

impl RetryContext for TimeoutRetryContext {
    fn set_exhausted_only(&self) {
        self.context_support.set_exhausted_only()
    }

    fn is_exhausted_only(&self) -> bool {
        self.context_support.is_exhausted_only()
    }

    fn get_parent(&self) -> Option<&dyn RetryContext> {
        self.context_support.get_parent()
    }

    fn get_retry_count(&self) -> u16 {
        self.context_support.get_retry_count()
    }

    fn get_last_error(&self) -> Option<RetryError> {
        self.context_support.get_last_error()
    }
}

impl Named for TimeoutRetryPolicy {
    fn name(&self) -> &str {
        "TimeoutRetryPolicy"
    }
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
