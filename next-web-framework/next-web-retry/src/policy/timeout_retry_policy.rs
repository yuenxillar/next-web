use std::any::Any;

use next_web_core::{convert::into_box::IntoBox, util::time::LocalTime};

use crate::{
    context::retry_context_support::RetryContextSupport,
    retry_context::{AttributeAccessor, RetryContext},
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

impl RetryPolicy for TimeoutRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {
        let any: &dyn Any = context;
        match any.downcast_ref::<TimeoutRetryContext>() {
            Some(context) => context.is_alive(),
            None => false,
        }
    }

    fn open(&self, context: Option<&dyn RetryContext>) -> Box<dyn RetryContext> {
        TimeoutRetryContext::new(self.timeout).into_boxed()
    }

    fn close(&self, _context: &dyn RetryContext) {}

    fn register_error(
        &self,
        context: &mut dyn RetryContext,
        error: Option<&dyn crate::error::AnyError>,
    ) {
        let context: &mut dyn Any = context;
        match context.downcast_mut::<RetryContextSupport>() {
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
            start: LocalTime::timestamp(),
            timeout,
        }
    }
    fn is_alive(&self) -> bool {
        (LocalTime::timestamp() - self.start) <= self.timeout
    }
}

impl AttributeAccessor for TimeoutRetryContext {
    fn has_attribute(&self, name: &str) -> bool {
        todo!()
    }

    fn set_attribute(&mut self, name: &str, value: next_web_core::util::any_map::AnyValue) {
        todo!()
    }

    fn remove_attribute(&mut self, name: &str) -> Option<next_web_core::util::any_map::AnyValue> {
        todo!()
    }

    fn get_attribute(&self, name: &str) -> Option<&next_web_core::util::any_map::AnyValue> {
        todo!()
    }
}

impl RetryContext for TimeoutRetryContext {
    fn set_exhausted_only(&mut self) {
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

    fn get_last_error(&self) -> Option<Box<dyn crate::error::AnyError>> {
        todo!()
    }
}
