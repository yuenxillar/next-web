use std::{any::Any, sync::Arc};

use next_web_core::async_trait;

use crate::{
    backoff::{
        back_off_context::BackOffContext, back_off_policy::BackOffPolicy,
        sleeping_back_off_policy::SleepingBackOffPolicy,
    },
    error::retry_error::RetryError,
};

#[derive(Clone)]
pub struct ExponentialBackOffPolicy {
    initial_interval: u64,
    max_interval: u64,
    multiplier: f32,
    with_random: bool,
}

impl ExponentialBackOffPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_random() -> Self {
        Self {
            with_random: true,
            ..Default::default()
        }
    }

    pub fn set_initial_interval(&mut self, initial_interval: u64) {
        self.initial_interval = initial_interval;
    }

    pub fn set_max_interval(&mut self, max_interval: u64) {
        self.max_interval = max_interval;
    }

    pub fn set_multiplier(&mut self, multiplier: f32) {
        self.multiplier = multiplier;
    }
}

#[async_trait]
impl BackOffPolicy for ExponentialBackOffPolicy {
    async fn start(
        &self,
        context: &dyn crate::retry_context::RetryContext,
    ) -> Option<Arc<dyn BackOffContext>> {
        Some(Arc::new(ExponentialBackOffContext {
            interval: todo!(),
            multiplier: todo!(),
            max_interval: todo!(),
        }))
    }

    async fn backoff(
        &self,
        context: Option<&dyn BackOffContext>,
    ) -> Result<(), crate::error::retry_error::RetryError> {
        if let Some(context) = context {
            let any: &dyn Any = context;
            match any.downcast_ref::<ExponentialBackOffContext>() {
                Some(ctx) => {
                    let sleep_time = ctx.get_sleep_and_increment();
                    self.sleep(sleep_time).await;
                }
                None => {}
            }
        }

        Ok(())
    }
}

#[async_trait]
impl SleepingBackOffPolicy for ExponentialBackOffPolicy {
    async fn sleep(&self, sleep: u64) {
        tokio::time::sleep(tokio::time::Duration::from_millis(sleep)).await;
    }
}

impl Default for ExponentialBackOffPolicy {
    fn default() -> Self {
        Self {
            initial_interval: Default::default(),
            max_interval: Default::default(),
            multiplier: Default::default(),
            with_random: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct ExponentialBackOffContext {
    interval: u64,
    multiplier: f32,
    max_interval: u64,
}

impl ExponentialBackOffContext {
    pub fn get_sleep_and_increment(&self) -> u64 {
        0
    }
}
