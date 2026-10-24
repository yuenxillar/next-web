//! A back off that grows exponentially up to a maximum interval.

use std::{
    any::Any,
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use next_web_core::async_trait;
use rand::Rng;
use tracing::warn;

use crate::{
    backoff::{
        back_off_context::BackOffContext,
        back_off_policy::BackOffPolicy,
        sleeper::{Sleeper, ThreadWaitSleeper},
        sleeping_back_off_policy::SleepingBackOffPolicy,
    },
    error::retry_error::RetryError,
    retry_context::RetryContext,
};

/// The default initial interval, in milliseconds.
pub const DEFAULT_INITIAL_INTERVAL: u64 = 100;

/// The default maximum interval, in milliseconds.
pub const DEFAULT_MAX_INTERVAL: u64 = 30_000;

/// The default multiplier.
pub const DEFAULT_MULTIPLIER: f64 = 2.0;

/// Grows the wait exponentially between attempts, up to a maximum.
///
/// The wait starts at the initial interval and is multiplied after every attempt until it
/// reaches the maximum interval, after which it stays there. The state of the schedule
/// lives in a [`ExponentialBackOffContext`], so one policy can serve many retry blocks at
/// once without sharing their progress.
#[derive(Clone)]
pub struct ExponentialBackOffPolicy {
    initial_interval: u64,
    max_interval: u64,
    multiplier: f64,
    with_random: bool,
    initial_interval_supplier: Option<Arc<dyn Fn() -> u64 + Send + Sync>>,
    max_interval_supplier: Option<Arc<dyn Fn() -> u64 + Send + Sync>>,
    multiplier_supplier: Option<Arc<dyn Fn() -> f64 + Send + Sync>>,
    sleeper: Arc<dyn Sleeper>,
}

impl ExponentialBackOffPolicy {
    /// Creates a policy with the default intervals and multiplier.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a policy whose waits are randomised with jitter.
    pub fn with_random() -> Self {
        Self {
            with_random: true,
            ..Self::default()
        }
    }

    /// Sets the wait of the first retry, in milliseconds.
    ///
    /// Values below one are clamped to one and reported, because a zero interval would make
    /// the growth meaningless.
    pub fn set_initial_interval(&mut self, initial_interval: u64) {
        if initial_interval < 1 {
            warn!(
                initial_interval,
                "Initial interval must be at least 1, clamping to 1"
            );
        }
        self.initial_interval = initial_interval.max(1);
    }

    /// Installs a dynamic source for the initial interval.
    pub fn set_initial_interval_supplier<F>(&mut self, supplier: F)
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        self.initial_interval_supplier = Some(Arc::new(supplier));
    }

    /// Returns the effective initial interval.
    pub fn get_initial_interval(&self) -> u64 {
        self.initial_interval_supplier
            .as_ref()
            .map(|supplier| supplier())
            .unwrap_or(self.initial_interval)
    }

    /// Sets the maximum wait, in milliseconds.
    pub fn set_max_interval(&mut self, max_interval: u64) {
        if max_interval < 1 {
            warn!(max_interval, "Max interval must be positive, clamping to 1");
        }
        self.max_interval = max_interval.max(1);
    }

    /// Installs a dynamic source for the maximum interval.
    pub fn set_max_interval_supplier<F>(&mut self, supplier: F)
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        self.max_interval_supplier = Some(Arc::new(supplier));
    }

    /// Returns the effective maximum interval.
    pub fn get_max_interval(&self) -> u64 {
        self.max_interval_supplier
            .as_ref()
            .map(|supplier| supplier())
            .unwrap_or(self.max_interval)
    }

    /// Sets the growth factor.
    ///
    /// Values at or below one do not grow the wait and are clamped to one, which turns the
    /// policy into a fixed wait.
    pub fn set_multiplier(&mut self, multiplier: f64) {
        if multiplier <= 1.0 {
            warn!(
                multiplier,
                "Multiplier must be greater than 1.0 for an exponential back off, clamping to 1.0"
            );
        }
        self.multiplier = if multiplier > 1.0 { multiplier } else { 1.0 };
    }

    /// Installs a dynamic source for the multiplier.
    pub fn set_multiplier_supplier<F>(&mut self, supplier: F)
    where
        F: Fn() -> f64 + Send + Sync + 'static,
    {
        self.multiplier_supplier = Some(Arc::new(supplier));
    }

    /// Returns the effective multiplier.
    pub fn get_multiplier(&self) -> f64 {
        self.multiplier_supplier
            .as_ref()
            .map(|supplier| supplier())
            .unwrap_or(self.multiplier)
    }

    /// Enables or disables jitter.
    pub fn set_with_random(&mut self, with_random: bool) {
        self.with_random = with_random;
    }

    /// Returns whether jitter is enabled.
    pub fn is_with_random(&self) -> bool {
        self.with_random
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
}

impl Default for ExponentialBackOffPolicy {
    fn default() -> Self {
        Self {
            initial_interval: DEFAULT_INITIAL_INTERVAL,
            max_interval: DEFAULT_MAX_INTERVAL,
            multiplier: DEFAULT_MULTIPLIER,
            with_random: false,
            initial_interval_supplier: None,
            max_interval_supplier: None,
            multiplier_supplier: None,
            sleeper: Arc::new(ThreadWaitSleeper::new()),
        }
    }
}

#[async_trait]
impl BackOffPolicy for ExponentialBackOffPolicy {
    async fn start(&self, _context: &dyn RetryContext) -> Option<Arc<dyn BackOffContext>> {
        Some(Arc::new(ExponentialBackOffContext::new(
            self.initial_interval,
            self.get_multiplier(),
            self.max_interval,
            self.with_random,
            self.initial_interval_supplier.clone(),
            self.multiplier_supplier.clone(),
            self.max_interval_supplier.clone(),
        )))
    }

    async fn backoff(&self, context: Option<&dyn BackOffContext>) -> Result<(), RetryError> {
        let Some(context) = context else {
            return Ok(());
        };
        let any: &dyn Any = context;
        if let Some(context) = any.downcast_ref::<ExponentialBackOffContext>() {
            let sleep_time = context.get_sleep_and_increment();
            self.sleep(sleep_time).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl SleepingBackOffPolicy for ExponentialBackOffPolicy {
    async fn sleep(&self, back_off_period: u64) -> Result<(), RetryError> {
        self.sleeper.sleep(back_off_period).await
    }
}

impl fmt::Display for ExponentialBackOffPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ExponentialBackOffPolicy[initialInterval={}, multiplier={}, maxInterval={}]",
            self.get_initial_interval(),
            self.get_multiplier(),
            self.get_max_interval()
        )
    }
}

/// The schedule of one exponential back off.
///
/// Each block of retries gets its own context: the current interval is advanced on every
/// wait, and the value handed to the sleeper is the interval *before* the advance.
pub struct ExponentialBackOffContext {
    interval: AtomicU64,
    multiplier: f64,
    max_interval: u64,
    with_random: bool,
    initial_interval_supplier: Mutex<Option<Arc<dyn Fn() -> u64 + Send + Sync>>>,
    multiplier_supplier: Option<Arc<dyn Fn() -> f64 + Send + Sync>>,
    max_interval_supplier: Option<Arc<dyn Fn() -> u64 + Send + Sync>>,
}

impl ExponentialBackOffContext {
    /// Creates a context with an explicit configuration.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        interval: u64,
        multiplier: f64,
        max_interval: u64,
        with_random: bool,
        initial_interval_supplier: Option<Arc<dyn Fn() -> u64 + Send + Sync>>,
        multiplier_supplier: Option<Arc<dyn Fn() -> f64 + Send + Sync>>,
        max_interval_supplier: Option<Arc<dyn Fn() -> u64 + Send + Sync>>,
    ) -> Self {
        Self {
            interval: AtomicU64::new(interval),
            multiplier,
            max_interval,
            with_random,
            initial_interval_supplier: Mutex::new(initial_interval_supplier),
            multiplier_supplier,
            max_interval_supplier,
        }
    }

    /// Returns the wait for the next attempt and advances the schedule.
    pub fn get_sleep_and_increment(&self) -> u64 {
        let interval = self.get_interval();
        let max = self.get_max_interval();
        let sleep = if interval > max {
            max
        } else {
            let next = self.calculate_next_interval(interval);
            self.interval.store(next, Ordering::Release);
            interval
        };

        if !self.with_random {
            return sleep;
        }

        let multiplier = self.get_multiplier();
        let jitter = 1.0 + rand::thread_rng().gen_range(0.0..1.0) * (multiplier - 1.0);
        ((sleep as f64 * jitter) as u64).min(max)
    }

    /// Returns the current interval, consuming a one-shot initial interval supplier.
    pub fn get_interval(&self) -> u64 {
        let supplier = self
            .initial_interval_supplier
            .lock()
            .ok()
            .and_then(|mut supplier| supplier.take());

        match supplier {
            Some(supplier) => {
                let interval = supplier();
                self.interval.store(interval, Ordering::Release);
                interval
            }
            None => self.interval.load(Ordering::Acquire),
        }
    }

    /// Returns the effective multiplier.
    pub fn get_multiplier(&self) -> f64 {
        self.multiplier_supplier
            .as_ref()
            .map(|supplier| supplier())
            .unwrap_or(self.multiplier)
    }

    /// Returns the effective maximum interval.
    pub fn get_max_interval(&self) -> u64 {
        self.max_interval_supplier
            .as_ref()
            .map(|supplier| supplier())
            .unwrap_or(self.max_interval)
    }

    /// Returns the interval the next wait would use.
    pub fn get_next_interval(&self) -> u64 {
        self.calculate_next_interval(self.interval.load(Ordering::Acquire))
    }

    fn calculate_next_interval(&self, interval: u64) -> u64 {
        (interval as f64 * self.get_multiplier()) as u64
    }
}

impl Clone for ExponentialBackOffContext {
    fn clone(&self) -> Self {
        Self {
            interval: AtomicU64::new(self.interval.load(Ordering::Acquire)),
            multiplier: self.multiplier,
            max_interval: self.max_interval,
            with_random: self.with_random,
            initial_interval_supplier: Mutex::new(
                self.initial_interval_supplier
                    .lock()
                    .ok()
                    .and_then(|supplier| supplier.clone()),
            ),
            multiplier_supplier: self.multiplier_supplier.clone(),
            max_interval_supplier: self.max_interval_supplier.clone(),
        }
    }
}

impl fmt::Debug for ExponentialBackOffContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExponentialBackOffContext")
            .field("interval", &self.get_interval())
            .field("multiplier", &self.get_multiplier())
            .field("max_interval", &self.get_max_interval())
            .field("with_random", &self.with_random)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{ExponentialBackOffContext, ExponentialBackOffPolicy};
    use crate::{
        backoff::{back_off_policy::BackOffPolicy, sleeper::RecordingSleeper},
        context::retry_context_support::RetryContextSupport,
    };

    fn deterministic_context(
        interval: u64,
        multiplier: f64,
        max_interval: u64,
    ) -> ExponentialBackOffContext {
        ExponentialBackOffContext::new(interval, multiplier, max_interval, false, None, None, None)
    }

    #[test]
    fn the_interval_grows_and_then_stays_at_the_maximum() {
        let context = deterministic_context(1_000, 1.5, 3_000);

        assert_eq!(context.get_sleep_and_increment(), 1_000);
        assert_eq!(context.get_sleep_and_increment(), 1_500);
        assert_eq!(context.get_sleep_and_increment(), 2_250);
        assert_eq!(context.get_sleep_and_increment(), 3_000);
        assert_eq!(context.get_sleep_and_increment(), 3_000);
    }

    #[test]
    fn growth_truncates_the_fractional_part() {
        let context = deterministic_context(1_001, 1.5, 10_000);

        assert_eq!(context.get_sleep_and_increment(), 1_001);
        assert_eq!(context.get_sleep_and_increment(), 1_501);
    }

    #[test]
    fn a_random_context_stays_within_the_configured_bounds() {
        let context = ExponentialBackOffContext::new(1_000, 2.0, 1_200, true, None, None, None);

        for _ in 0..100 {
            let sleep = context.get_sleep_and_increment();
            assert!((1_000..=1_200).contains(&sleep), "sleep was {sleep}");
        }
    }

    #[tokio::test]
    async fn the_policy_uses_the_configured_intervals() {
        let recorder = Arc::new(RecordingSleeper::new());
        let mut policy = ExponentialBackOffPolicy::new();
        policy.set_sleeper(recorder.clone());
        policy.set_initial_interval(10);
        policy.set_multiplier(2.0);
        policy.set_max_interval(25);
        let context = RetryContextSupport::new();
        let back_off_context = policy.start(&context).await;

        policy
            .backoff(back_off_context.as_deref())
            .await
            .expect("recording cannot fail");
        policy
            .backoff(back_off_context.as_deref())
            .await
            .expect("recording cannot fail");
        policy
            .backoff(back_off_context.as_deref())
            .await
            .expect("recording cannot fail");

        assert_eq!(recorder.periods(), vec![10, 20, 25]);
    }

    #[test]
    fn the_defaults_match_the_documented_values() {
        let policy = ExponentialBackOffPolicy::new();

        assert_eq!(policy.get_initial_interval(), 100);
        assert_eq!(policy.get_max_interval(), 30_000);
        assert_eq!(policy.get_multiplier(), 2.0);
        assert!(!policy.is_with_random());
    }

    #[test]
    fn a_multiplier_at_or_below_one_is_clamped() {
        let mut policy = ExponentialBackOffPolicy::new();
        policy.set_multiplier(0.5);

        assert_eq!(policy.get_multiplier(), 1.0);
    }
}
