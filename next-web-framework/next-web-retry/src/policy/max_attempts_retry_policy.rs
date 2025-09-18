use std::{any::Any, sync::{atomic::{AtomicU16, Ordering}, Arc}};

use next_web_core::convert::into_box::IntoBox;

use crate::{context::retry_context_support::RetryContextSupport, retry_context::RetryContext, retry_policy::RetryPolicy};

#[derive(Clone)]
pub struct MaxAttemptsRetryPolicy {
    max_attempts: Arc<AtomicU16>,
}

impl MaxAttemptsRetryPolicy {
    pub fn new(max_attempts: u16) -> Self {
        MaxAttemptsRetryPolicy {
            max_attempts: Arc::new(AtomicU16::new(max_attempts)),
        }
    }

    pub fn set_max_attempts(&mut self, max_attempts: u16) {
        self.max_attempts = Arc::new(AtomicU16::new(max_attempts));
    }
}

impl Default for MaxAttemptsRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: Arc::new(AtomicU16::new(3)),
        }
    }
}

impl RetryPolicy for MaxAttemptsRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {
        context.get_retry_count() < self.get_max_attempts()
    }

    fn open(
        &self,
        _context: Option<&dyn RetryContext>,
    ) -> Arc<dyn RetryContext> {
        // TODO parent
        Arc::new(RetryContextSupport::default())
    }

    fn close(&self, _context: &dyn RetryContext) {}

    fn register_error(
        &self,
        context: &dyn RetryContext,
        error: Option<&dyn crate::error::AnyError>,
    ) {
        // TODO parent
        let any: &dyn Any = context;
        if let Some(ctx) = any.downcast_ref::<RetryContextSupport>() {
            ctx.register_error(error);
        }
    }

    fn get_max_attempts(&self) -> u16 {
        self.max_attempts.load(Ordering::Relaxed)
    }
}
