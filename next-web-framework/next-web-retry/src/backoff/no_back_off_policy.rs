use crate::{
    backoff::{back_off_context::BackOffContext, back_off_policy::BackOffPolicy},
    error::back_off_interrupted_error::BackOffInterruptedError,
    retry_context::RetryContext,
};

pub struct NoBackOffPolicy {}

impl BackOffPolicy for NoBackOffPolicy {
    fn start(&self, _context: &dyn RetryContext) -> Option<std::sync::Arc<dyn BackOffContext>> {
        None
    }

    fn backoff(&self, _context: &dyn BackOffContext) -> Result<(), BackOffInterruptedError> {
        Ok(())
    }
}

impl Default for NoBackOffPolicy {
    fn default() -> Self {
        Self {  }
    }
}

impl ToString for NoBackOffPolicy {
    fn to_string(&self) -> String {
        String::from("NoBackOffPolicy []")
    }
}
