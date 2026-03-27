use std::{any::Any, sync::Arc};

use next_web_core::async_trait;
use rand::Rng;

use crate::backoff::{
    back_off_context::BackOffContext, sleeping_back_off_policy::SleepingBackOffPolicy,
};

use super::back_off_policy::BackOffPolicy;

#[derive(Clone)]
pub struct UniformRandomBackOffPolicy {
    min_back_off_period: u64,
    max_back_off_period: u64,
}

impl UniformRandomBackOffPolicy {
    pub fn new() -> Self {
        Self {
            min_back_off_period: 500,
            max_back_off_period: 1500,
        }
    }

    pub fn set_min_back_off_period(&mut self, min_interval: u64) {
        self.min_back_off_period = min_interval;
    }

    pub fn set_max_back_off_period(&mut self, max_interval: u64) {
        self.max_back_off_period = max_interval;
    }
}

#[async_trait]
impl BackOffPolicy for UniformRandomBackOffPolicy {
    async fn start(
        &self,
        _context: &dyn crate::retry_context::RetryContext,
    ) -> Option<Arc<dyn BackOffContext>> {
        Some(Arc::new(UniformRandomBackOffContext {
            min_back_off_period: self.min_back_off_period,
            max_back_off_period: self.max_back_off_period,
        }))
    }

    async fn backoff(
        &self,
        context: Option<&dyn BackOffContext>,
    ) -> Result<(), crate::error::retry_error::RetryError> {
        if let Some(context) = context {
            let any: &dyn Any = context;
            if let Some(context) = any.downcast_ref::<UniformRandomBackOffContext>() {
                let sleep = if context.min_back_off_period >= context.max_back_off_period {
                    context.min_back_off_period
                } else {
                    rand::thread_rng()
                        .gen_range(context.min_back_off_period..=context.max_back_off_period)
                };
                self.sleep(sleep).await;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl SleepingBackOffPolicy for UniformRandomBackOffPolicy {
    async fn sleep(&self, sleep: u64) {
        tokio::time::sleep(tokio::time::Duration::from_millis(sleep)).await;
    }
}

#[derive(Clone)]
struct UniformRandomBackOffContext {
    min_back_off_period: u64,
    max_back_off_period: u64,
}

impl BackOffContext for UniformRandomBackOffContext {
    fn get_value(&self) -> Option<&next_web_core::anys::any_value::AnyValue> {
        None
    }
}
