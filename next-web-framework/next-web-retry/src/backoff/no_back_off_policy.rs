use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    backoff::{back_off_context::BackOffContext, back_off_policy::BackOffPolicy},
    error::retry_error::RetryError,
    retry_context::RetryContext,
};

pub struct NoBackOffPolicy {}


impl NoBackOffPolicy {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl BackOffPolicy for NoBackOffPolicy {
    async fn start(&self, _context: &dyn RetryContext) -> Option<Arc<dyn BackOffContext>> {
        None
    }

    async fn backoff(&self, _context: Option<&dyn BackOffContext>,) -> Result<(), RetryError> {
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
