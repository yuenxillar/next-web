use std::sync::{atomic::{AtomicU16, Ordering}, Arc};

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
        context: Option<&dyn RetryContext>,
    ) -> Box<dyn RetryContext> {
        RetryContextSupport::new().into_boxed()
    }

    fn close(&self, _context: &dyn RetryContext) {}

    fn register_error(
        &self,
        context: &mut dyn RetryContext,
        error: Option<&dyn crate::error::AnyError>,
    ) {
        todo!()
    }

    fn get_max_attempts(&self) -> u16 {
        self.max_attempts.load(Ordering::Relaxed)
    }
}
