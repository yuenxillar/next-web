use std::{any::Any, sync::{atomic::{AtomicU64, Ordering}, Arc}};

use next_web_core::async_trait;
use tracing::warn;

use crate::backoff::{
        back_off_context::BackOffContext, back_off_policy::BackOffPolicy,
        sleeping_back_off_policy::SleepingBackOffPolicy,
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
        if initial_interval < 1 {
            warn!("Initial interval must be at least 1, but was {initial_interval}");
        }
        self.initial_interval = if initial_interval > 1 { initial_interval } else { 1 };
    }

    pub fn set_max_interval(&mut self, max_interval: u64) {
        if max_interval < 1 {
            warn!("Max interval must be positive, but was  {max_interval}");
        }
        self.max_interval = if max_interval > 0 { max_interval } else { 1 };
    }

    pub fn set_multiplier(&mut self, multiplier: f32) {
        if multiplier <= 1.0 {
            warn!("Multiplier must be > 1.0 for effective exponential backoff, but was {multiplier}");
        }
        self.multiplier = if multiplier > 1.0 { multiplier } else { 1.0 };
    }

    pub fn get_initial_interval(&self) -> u64 {
        self.initial_interval
    }

    pub fn get_max_interval(&self) -> u64 {
        self.max_interval
    }

    pub fn get_multiplier(&self) -> f32 {
        self.multiplier
    }
}

#[async_trait]
impl BackOffPolicy for ExponentialBackOffPolicy {
    async fn start(
        &self,
        _context: &dyn crate::retry_context::RetryContext,
    ) -> Option<Arc<dyn BackOffContext>> {
        Some(Arc::new(ExponentialBackOffContext {
            interval: Arc::new(AtomicU64::new(self.initial_interval)),
            multiplier: self.multiplier,
            max_interval: self.max_interval,
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
    interval: Arc<AtomicU64>,
    multiplier: f32,
    max_interval: u64,
}

impl ExponentialBackOffContext {
    pub fn get_sleep_and_increment(&self) -> u64 {
        
        let mut sleep = self.get_interval();
        let max = self.get_max_interval();

        if sleep > max {
            sleep = max;
        }else {
            self.interval.store(self.get_next_interval(), Ordering::Relaxed);
        };
        
        // TODO random
        sleep
    }

    pub fn get_interval(&self) -> u64 {
        self.interval.load(Ordering::Relaxed)
    }

    pub fn get_multiplier(&self) -> f32 {
        self.multiplier
    }

    pub fn get_next_interval(&self) -> u64 {
        self.interval.load(Ordering::Relaxed) * (self.multiplier as u64)
    }

    pub fn get_max_interval(&self) -> u64 {
        self.max_interval
    }
}

impl BackOffContext for ExponentialBackOffContext {
    fn get_value(&self) -> Option<&next_web_core::util::any_map::AnyValue> {
        None
    }
}