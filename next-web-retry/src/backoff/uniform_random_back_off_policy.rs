//! A back off that waits a uniformly random period between two bounds.

use std::{fmt, sync::Arc};

use next_web_core::async_trait;
use rand::Rng;

use crate::{
    backoff::{
        back_off_context::BackOffContext,
        back_off_policy::BackOffPolicy,
        sleeper::{Sleeper, ThreadWaitSleeper},
        sleeping_back_off_policy::SleepingBackOffPolicy,
        stateless_back_off_policy::{StatelessBackOffPolicy, stateless_backoff, stateless_start},
    },
    error::retry_error::RetryError,
    retry_context::RetryContext,
};

/// The default minimum back off period, in milliseconds.
pub const DEFAULT_BACK_OFF_MIN_PERIOD: u64 = 500;

/// The default maximum back off period, in milliseconds.
pub const DEFAULT_BACK_OFF_MAX_PERIOD: u64 = 1_500;

/// Waits for a random period between two bounds before every retry.
///
/// The randomness spreads concurrent retries that failed together, so they do not all
/// hammer the recovered dependency at the same instant.
#[derive(Clone)]
pub struct UniformRandomBackOffPolicy {
    min_back_off_period: Arc<dyn Fn() -> u64 + Send + Sync>,
    max_back_off_period: Arc<dyn Fn() -> u64 + Send + Sync>,
    sleeper: Arc<dyn Sleeper>,
}

impl UniformRandomBackOffPolicy {
    /// Creates a policy that waits between [`DEFAULT_BACK_OFF_MIN_PERIOD`] and
    /// [`DEFAULT_BACK_OFF_MAX_PERIOD`] milliseconds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the sleeper.
    pub fn set_sleeper(&mut self, sleeper: Arc<dyn Sleeper>) {
        self.sleeper = sleeper;
    }

    /// Replaces the sleeper, returning the policy for chaining.
    pub fn with_sleeper(mut self, sleeper: Arc<dyn Sleeper>) -> Self {
        self.sleeper = sleeper;
        self
    }

    /// Sets the minimum wait in milliseconds.
    pub fn set_min_back_off_period(&mut self, back_off_period: u64) {
        self.min_back_off_period = Arc::new(move || back_off_period.max(1));
    }

    /// Installs a dynamic source for the minimum wait.
    pub fn set_min_back_off_period_supplier<F>(&mut self, supplier: F)
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        self.min_back_off_period = Arc::new(supplier);
    }

    /// Returns the configured minimum wait in milliseconds.
    pub fn get_min_back_off_period(&self) -> u64 {
        (self.min_back_off_period)()
    }

    /// Sets the maximum wait in milliseconds.
    pub fn set_max_back_off_period(&mut self, back_off_period: u64) {
        self.max_back_off_period = Arc::new(move || back_off_period.max(1));
    }

    /// Installs a dynamic source for the maximum wait.
    pub fn set_max_back_off_period_supplier<F>(&mut self, supplier: F)
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        self.max_back_off_period = Arc::new(supplier);
    }

    /// Returns the configured maximum wait in milliseconds.
    pub fn get_max_back_off_period(&self) -> u64 {
        (self.max_back_off_period)()
    }

    /// Returns the wait to use for the next attempt.
    ///
    /// The value is uniform in `[min, max)`; a maximum that is not above the minimum is
    /// treated as a fixed wait, which is what the original implementation does.
    fn next_period(&self) -> u64 {
        let min = self.get_min_back_off_period();
        let max = self.get_max_back_off_period();
        if max <= min {
            min
        } else {
            rand::thread_rng().gen_range(min..max)
        }
    }
}

impl Default for UniformRandomBackOffPolicy {
    fn default() -> Self {
        Self {
            min_back_off_period: Arc::new(|| DEFAULT_BACK_OFF_MIN_PERIOD),
            max_back_off_period: Arc::new(|| DEFAULT_BACK_OFF_MAX_PERIOD),
            sleeper: Arc::new(ThreadWaitSleeper::new()),
        }
    }
}

#[async_trait]
impl BackOffPolicy for UniformRandomBackOffPolicy {
    async fn start(&self, context: &dyn RetryContext) -> Option<Arc<dyn BackOffContext>> {
        stateless_start(context).await
    }

    async fn backoff(&self, _context: Option<&dyn BackOffContext>) -> Result<(), RetryError> {
        stateless_backoff(self).await
    }
}

#[async_trait]
impl StatelessBackOffPolicy for UniformRandomBackOffPolicy {
    async fn do_backoff(&self) -> Result<(), RetryError> {
        self.sleep(self.next_period()).await
    }
}

#[async_trait]
impl SleepingBackOffPolicy for UniformRandomBackOffPolicy {
    async fn sleep(&self, back_off_period: u64) -> Result<(), RetryError> {
        self.sleeper.sleep(back_off_period).await
    }
}

impl fmt::Display for UniformRandomBackOffPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UniformRandomBackOffPolicy[minBackOffPeriod={}, maxBackOffPeriod={}]",
            self.get_min_back_off_period(),
            self.get_max_back_off_period()
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::UniformRandomBackOffPolicy;
    use crate::{
        backoff::{back_off_policy::BackOffPolicy, sleeper::RecordingSleeper},
        context::retry_context_support::RetryContextSupport,
    };

    #[tokio::test]
    async fn every_wait_stays_within_the_bounds() {
        let recorder = Arc::new(RecordingSleeper::new());
        let mut policy = UniformRandomBackOffPolicy::new();
        policy.set_sleeper(recorder.clone());
        policy.set_min_back_off_period(10);
        policy.set_max_back_off_period(20);
        let context = RetryContextSupport::new();

        assert!(policy.start(&context).await.is_none());
        for _ in 0..50 {
            policy.backoff(None).await.expect("recording cannot fail");
        }

        let periods = recorder.periods();
        assert_eq!(periods.len(), 50);
        assert!(periods.iter().all(|period| (10..20).contains(period)));
    }

    #[tokio::test]
    async fn a_maximum_below_the_minimum_is_a_fixed_wait() {
        let recorder = Arc::new(RecordingSleeper::new());
        let mut policy = UniformRandomBackOffPolicy::new();
        policy.set_sleeper(recorder.clone());
        policy.set_min_back_off_period(30);
        policy.set_max_back_off_period(30);

        policy.backoff(None).await.expect("recording cannot fail");

        assert_eq!(recorder.periods(), vec![30]);
    }
}
