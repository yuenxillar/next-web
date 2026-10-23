//! An exponential back off randomised with jitter.

use std::{ops::Deref, sync::Arc};

use next_web_core::async_trait;

use crate::{
    backoff::{
        back_off_context::BackOffContext,
        back_off_policy::BackOffPolicy,
        exponential_back_off_policy::{ExponentialBackOffContext, ExponentialBackOffPolicy},
        sleeper::Sleeper,
        sleeping_back_off_policy::SleepingBackOffPolicy,
    },
    error::retry_error::RetryError,
    retry_context::RetryContext,
};

/// An exponential back off with jitter.
///
/// The deterministic exponential wait is multiplied by a random factor between one and the
/// configured multiplier, then capped at the maximum interval. The result is spread over
/// `[interval, interval * multiplier)`, which prevents a set of retries that failed together
/// from waking together.
///
/// The example of the original documentation: an initial interval of 50 and a multiplier of
/// 2 produce deterministic waits of `[50, 100, 200, 400]`, while this policy produces values
/// within `[50-100, 100-200, 200-400, 400-800]`.
#[derive(Clone)]
pub struct ExponentialRandomBackOffPolicy {
    inner: ExponentialBackOffPolicy,
}

impl ExponentialRandomBackOffPolicy {
    /// Creates a policy with the default intervals and multiplier, with jitter enabled.
    pub fn new() -> Self {
        Self {
            inner: ExponentialBackOffPolicy::with_random(),
        }
    }

    /// Returns the underlying policy configuration.
    pub fn inner(&self) -> &ExponentialBackOffPolicy {
        &self.inner
    }

    /// Returns the underlying policy configuration for mutation.
    pub fn inner_mut(&mut self) -> &mut ExponentialBackOffPolicy {
        &mut self.inner
    }
}

impl Default for ExponentialRandomBackOffPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for ExponentialRandomBackOffPolicy {
    type Target = ExponentialBackOffPolicy;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::fmt::Display for ExponentialRandomBackOffPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExponentialRandomBackOffPolicy[{}]", self.inner)
    }
}

#[async_trait]
impl BackOffPolicy for ExponentialRandomBackOffPolicy {
    async fn start(&self, context: &dyn RetryContext) -> Option<Arc<dyn BackOffContext>> {
        self.inner.start(context).await
    }

    async fn backoff(&self, context: Option<&dyn BackOffContext>) -> Result<(), RetryError> {
        self.inner.backoff(context).await
    }
}

#[async_trait]
impl SleepingBackOffPolicy for ExponentialRandomBackOffPolicy {
    async fn sleep(&self, back_off_period: u64) -> Result<(), RetryError> {
        self.inner.sleep(back_off_period).await
    }
}

impl ExponentialRandomBackOffPolicy {
    /// Replaces the sleeper.
    pub fn set_sleeper(&mut self, sleeper: Arc<dyn Sleeper>) {
        self.inner.set_sleeper(sleeper);
    }
}

/// Re-exports the context type of the deterministic policy so callers that build their own
/// policy do not need to import two modules.
pub type RandomBackOffContext = ExponentialBackOffContext;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::ExponentialRandomBackOffPolicy;
    use crate::{
        backoff::{back_off_policy::BackOffPolicy, sleeper::RecordingSleeper},
        context::retry_context_support::RetryContextSupport,
    };

    #[tokio::test]
    async fn every_wait_stays_between_the_interval_and_the_capped_next_interval() {
        let recorder = Arc::new(RecordingSleeper::new());
        let mut policy = ExponentialRandomBackOffPolicy::new();
        policy.set_sleeper(recorder.clone());
        policy.inner_mut().set_initial_interval(10);
        policy.inner_mut().set_multiplier(2.0);
        policy.inner_mut().set_max_interval(100);
        let context = RetryContextSupport::new();
        let back_off_context = policy.start(&context).await;

        for _ in 0..20 {
            policy
                .backoff(back_off_context.as_deref())
                .await
                .expect("recording cannot fail");
        }

        let periods = recorder.periods();
        assert_eq!(periods.len(), 20);
        assert!(periods.iter().all(|period| (10..=100).contains(period)));
    }

    #[test]
    fn the_policy_reports_itself_as_random() {
        let policy = ExponentialRandomBackOffPolicy::new();

        assert!(policy.is_with_random());
        assert!(
            policy
                .to_string()
                .starts_with("ExponentialRandomBackOffPolicy")
        );
    }
}
