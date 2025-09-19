use std::sync::Arc;

use next_web_core::async_trait;

use crate::backoff::{
    back_off_context::BackOffContext, back_off_policy::BackOffPolicy,
    sleeping_back_off_policy::SleepingBackOffPolicy,
};

#[derive(Clone)]
pub struct FixedBackOffPolicy {
    back_off_period: u64,
}
impl FixedBackOffPolicy {
    pub fn new() -> Self {
        Self {
            back_off_period: 1000,
        }
    }

    pub fn set_back_off_period(&mut self, back_off_period: u64) {
        self.back_off_period = if back_off_period > 0 {
            back_off_period
        } else {
            1
        };
    }
}

#[async_trait]
impl BackOffPolicy for FixedBackOffPolicy {
    async fn start(
        &self,
        _context: &dyn crate::retry_context::RetryContext,
    ) -> Option<Arc<dyn BackOffContext>> {
        None
    }

    async fn backoff(
        &self,
        _context: Option<&dyn BackOffContext>,
    ) -> Result<(), crate::error::retry_error::RetryError> {
        self.sleep(self.back_off_period).await;
        Ok(())
    }
}

#[async_trait]
impl SleepingBackOffPolicy for FixedBackOffPolicy {
    async fn sleep(&self, sleep: u64) {
        tokio::time::sleep(tokio::time::Duration::from_millis(sleep)).await;
    }
}
