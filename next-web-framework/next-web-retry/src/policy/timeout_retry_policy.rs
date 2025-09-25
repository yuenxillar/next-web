use std::{any::Any, sync::Arc};

use next_web_core::{async_trait, models::any_value::AnyValue};

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

    fn open(&self, context: Option<&dyn RetryContext>) -> Arc<dyn RetryContext> {
        Arc::new(TimeoutRetryContext::new(self.timeout))
    }

    fn close(&self, _context: &dyn RetryContext) {}

    fn register_error(
        &self,
        context: &dyn RetryContext,
        error: Option<&dyn crate::error::AnyError>,
    ) {
        let context: &dyn Any = context;
        match context.downcast_ref::<RetryContextSupport>() {
            Some(context) => context.register_error(error),
            None => {}
        }
    }
}

#[derive(Clone)]
struct TimeoutRetryContext {
    timeout: u64,
    start: u64,
}

impl TimeoutRetryContext {
    pub fn new(timeout: u64) -> Self {
        Self {
            start: timestamp(),
            timeout,
        }
    }
    fn is_alive(&self) -> bool {
        (timestamp() - self.start) <= self.timeout
    }
}

impl SyncAttributeAccessor for TimeoutRetryContext {
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

impl RetryContext for TimeoutRetryContext {
    fn set_exhausted_only(&self) {
        todo!()
    }

    fn is_exhausted_only(&self) -> bool {
        todo!()
    }

    fn get_parent(&self) -> Option<&dyn RetryContext> {
        todo!()
    }

    fn get_retry_count(&self) -> u16 {
        todo!()
    }

    fn get_last_error(&self) -> Option<RetryError> {
        todo!()
    }
}

impl ToString for TimeoutRetryPolicy {
    fn to_string(&self) -> String {
        "TimeoutRetryPolicy".to_string()
    }
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
