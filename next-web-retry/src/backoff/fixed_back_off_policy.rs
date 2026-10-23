//! A back off that waits the same period before every retry.

use std::{fmt, sync::Arc};

use next_web_core::async_trait;

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

/// The default back off period, in milliseconds.
pub const DEFAULT_BACK_OFF_PERIOD: u64 = 1_000;

/// Waits for a fixed period before every retry.
#[derive(Clone)]
pub struct FixedBackOffPolicy {
    back_off_period: Arc<dyn Fn() -> u64 + Send + Sync>,
    sleeper: Arc<dyn Sleeper>,
}

impl FixedBackOffPolicy {
    /// Creates a policy that waits for [`DEFAULT_BACK_OFF_PERIOD`] milliseconds.
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

    /// Sets the wait in milliseconds.
    ///
    /// Values below one are clamped to one, because a zero wait makes the configured pause
    /// meaningless and would spin instead of backing off.
    pub fn set_back_off_period(&mut self, back_off_period: u64) {
        self.back_off_period = Arc::new(move || back_off_period.max(1));
    }

    /// Installs a dynamic source for the wait.
    pub fn set_back_off_period_supplier<F>(&mut self, supplier: F)
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        self.back_off_period = Arc::new(supplier);
    }

    /// Returns the configured wait in milliseconds.
    pub fn get_back_off_period(&self) -> u64 {
        (self.back_off_period)()
    }
}

impl Default for FixedBackOffPolicy {
    fn default() -> Self {
        Self {
            back_off_period: Arc::new(|| DEFAULT_BACK_OFF_PERIOD),
            sleeper: Arc::new(ThreadWaitSleeper::new()),
        }
    }
}

#[async_trait]
impl BackOffPolicy for FixedBackOffPolicy {
    async fn start(&self, context: &dyn RetryContext) -> Option<Arc<dyn BackOffContext>> {
        stateless_start(context).await
    }

    async fn backoff(&self, _context: Option<&dyn BackOffContext>) -> Result<(), RetryError> {
        stateless_backoff(self).await
    }
}

#[async_trait]
impl StatelessBackOffPolicy for FixedBackOffPolicy {
    async fn do_backoff(&self) -> Result<(), RetryError> {
        self.sleep(self.get_back_off_period()).await
    }
}

#[async_trait]
impl SleepingBackOffPolicy for FixedBackOffPolicy {
    async fn sleep(&self, back_off_period: u64) -> Result<(), RetryError> {
        self.sleeper.sleep(back_off_period).await
    }
}

impl fmt::Display for FixedBackOffPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FixedBackOffPolicy[backOffPeriod={}]",
            self.get_back_off_period()
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::FixedBackOffPolicy;
    use crate::{
        backoff::{back_off_policy::BackOffPolicy, sleeper::RecordingSleeper},
        context::retry_context_support::RetryContextSupport,
    };

    #[tokio::test]
    async fn every_wait_uses_the_configured_period() {
        let recorder = Arc::new(RecordingSleeper::new());
        let mut policy = FixedBackOffPolicy::new();
        policy.set_sleeper(recorder.clone());
        policy.set_back_off_period(25);
        let context = RetryContextSupport::new();

        assert!(policy.start(&context).await.is_none());
        policy.backoff(None).await.expect("recording cannot fail");
        policy.backoff(None).await.expect("recording cannot fail");

        assert_eq!(policy.get_back_off_period(), 25);
        assert_eq!(recorder.periods(), vec![25, 25]);
    }

    #[tokio::test]
    async fn a_zero_period_is_clamped_to_one_millisecond() {
        let mut policy = FixedBackOffPolicy::new();
        policy.set_back_off_period(0);

        assert_eq!(policy.get_back_off_period(), 1);
    }

    #[test]
    fn display_reports_the_configured_period() {
        let mut policy = FixedBackOffPolicy::new();
        policy.set_back_off_period(7);

        assert_eq!(policy.to_string(), "FixedBackOffPolicy[backOffPeriod=7]");
    }
}
