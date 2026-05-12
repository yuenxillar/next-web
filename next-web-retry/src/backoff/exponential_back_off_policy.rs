use std::{
    any::Any,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use next_web_core::{anys::any_value::AnyValue, async_trait};
use rand::Rng;
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
        self.initial_interval = if initial_interval > 1 {
            initial_interval
        } else {
            1
        };
    }

    pub fn set_max_interval(&mut self, max_interval: u64) {
        if max_interval < 1 {
            warn!("Max interval must be positive, but was  {max_interval}");
        }
        self.max_interval = if max_interval > 0 { max_interval } else { 1 };
    }

    pub fn set_multiplier(&mut self, multiplier: f32) {
        if multiplier <= 1.0 {
            warn!(
                "Multiplier must be > 1.0 for effective exponential backoff, but was {multiplier}"
            );
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
            with_random: self.with_random,
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
            initial_interval: 1000,
            max_interval: 30000,
            multiplier: 2.0,
            with_random: false,
        }
    }
}

#[derive(Clone)]
pub struct ExponentialBackOffContext {
    interval: Arc<AtomicU64>,
    multiplier: f32,
    max_interval: u64,
    with_random: bool,
}

impl ExponentialBackOffContext {
    pub fn get_sleep_and_increment(&self) -> u64 {
        let interval = self.get_interval();
        let mut sleep = interval;
        let max = self.get_max_interval();

        if sleep > max {
            sleep = max;
        } else {
            self.interval
                .store(self.get_next_interval(), Ordering::Relaxed);
        };

        if self.with_random {
            let next = self.calculate_next_interval(interval).min(max);
            if sleep < next {
                return rand::thread_rng().gen_range(sleep..=next);
            }
        }

        sleep
    }

    pub fn get_interval(&self) -> u64 {
        self.interval.load(Ordering::Relaxed)
    }

    pub fn get_multiplier(&self) -> f32 {
        self.multiplier
    }

    pub fn get_next_interval(&self) -> u64 {
        self.calculate_next_interval(self.interval.load(Ordering::Relaxed))
    }

    pub fn get_max_interval(&self) -> u64 {
        self.max_interval
    }

    fn calculate_next_interval(&self, interval: u64) -> u64 {
        ((interval as f32) * self.multiplier).ceil() as u64
    }
}

impl BackOffContext for ExponentialBackOffContext {
    fn get_value(&self) -> Option<&AnyValue> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_random_enables_randomized_backoff() {
        let policy = ExponentialBackOffPolicy::with_random();

        assert!(policy.with_random);
    }

    #[test]
    fn deterministic_backoff_keeps_fractional_multiplier() {
        let context = ExponentialBackOffContext {
            interval: Arc::new(AtomicU64::new(1000)),
            multiplier: 1.5,
            max_interval: 10_000,
            with_random: false,
        };

        assert_eq!(context.get_sleep_and_increment(), 1000);
        assert_eq!(context.get_sleep_and_increment(), 1500);
        assert_eq!(context.get_sleep_and_increment(), 2250);
    }

    #[test]
    fn randomized_backoff_stays_between_current_interval_and_capped_next_interval() {
        let context = ExponentialBackOffContext {
            interval: Arc::new(AtomicU64::new(1000)),
            multiplier: 2.0,
            max_interval: 1500,
            with_random: true,
        };

        let sleep = context.get_sleep_and_increment();

        assert!((1000..=1500).contains(&sleep));
        assert_eq!(context.get_interval(), 2000);
    }
}
