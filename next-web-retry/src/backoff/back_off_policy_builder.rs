//! A fluent builder that selects a back-off policy from a delay, a maximum delay and a multiplier.

use std::{fmt, sync::Arc};

use next_web_core::async_trait;

use crate::{
    backoff::{
        back_off_context::BackOffContext,
        back_off_policy::BackOffPolicy,
        exponential_back_off_policy::{DEFAULT_MAX_INTERVAL, ExponentialBackOffPolicy},
        exponential_random_back_off_policy::ExponentialRandomBackOffPolicy,
        fixed_back_off_policy::{DEFAULT_BACK_OFF_PERIOD, FixedBackOffPolicy},
        sleeper::Sleeper,
        sleeping_back_off_policy::SleepingBackOffPolicy,
        uniform_random_back_off_policy::UniformRandomBackOffPolicy,
    },
    error::retry_error::RetryError,
    retry_context::RetryContext,
};

/// The concrete policy produced by [`BackOffPolicyBuilder`].
///
/// The builder chooses between four policies at build time; this enum lets it return one
/// value that still behaves as a [`BackOffPolicy`] and as a [`SleepingBackOffPolicy`].
#[derive(Clone)]
pub enum BuiltBackOffPolicy {
    /// A fixed wait.
    Fixed(FixedBackOffPolicy),
    /// A uniform random wait.
    UniformRandom(UniformRandomBackOffPolicy),
    /// An exponential wait.
    Exponential(ExponentialBackOffPolicy),
    /// An exponential wait with jitter.
    ExponentialRandom(ExponentialRandomBackOffPolicy),
}

#[async_trait]
impl BackOffPolicy for BuiltBackOffPolicy {
    async fn start(&self, context: &dyn RetryContext) -> Option<Arc<dyn BackOffContext>> {
        match self {
            BuiltBackOffPolicy::Fixed(policy) => policy.start(context).await,
            BuiltBackOffPolicy::UniformRandom(policy) => policy.start(context).await,
            BuiltBackOffPolicy::Exponential(policy) => policy.start(context).await,
            BuiltBackOffPolicy::ExponentialRandom(policy) => policy.start(context).await,
        }
    }

    async fn backoff(&self, context: Option<&dyn BackOffContext>) -> Result<(), RetryError> {
        match self {
            BuiltBackOffPolicy::Fixed(policy) => policy.backoff(context).await,
            BuiltBackOffPolicy::UniformRandom(policy) => policy.backoff(context).await,
            BuiltBackOffPolicy::Exponential(policy) => policy.backoff(context).await,
            BuiltBackOffPolicy::ExponentialRandom(policy) => policy.backoff(context).await,
        }
    }
}

#[async_trait]
impl SleepingBackOffPolicy for BuiltBackOffPolicy {
    async fn sleep(&self, back_off_period: u64) -> Result<(), RetryError> {
        match self {
            BuiltBackOffPolicy::Fixed(policy) => policy.sleep(back_off_period).await,
            BuiltBackOffPolicy::UniformRandom(policy) => policy.sleep(back_off_period).await,
            BuiltBackOffPolicy::Exponential(policy) => policy.sleep(back_off_period).await,
            BuiltBackOffPolicy::ExponentialRandom(policy) => policy.sleep(back_off_period).await,
        }
    }
}

impl fmt::Display for BuiltBackOffPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuiltBackOffPolicy::Fixed(policy) => write!(f, "{policy}"),
            BuiltBackOffPolicy::UniformRandom(policy) => write!(f, "{policy}"),
            BuiltBackOffPolicy::Exponential(policy) => write!(f, "{policy}"),
            BuiltBackOffPolicy::ExponentialRandom(policy) => write!(f, "{policy}"),
        }
    }
}

type DelaySupplier = Arc<dyn Fn() -> u64 + Send + Sync>;
type MultiplierSupplier = Arc<dyn Fn() -> f64 + Send + Sync>;
type RandomSupplier = Arc<dyn Fn() -> bool + Send + Sync>;

/// Builds a [`BackOffPolicy`] from a delay, an optional maximum delay and an optional
/// multiplier.
///
/// The shape of the resulting policy follows the configured values:
///
/// * a positive multiplier produces an exponential policy, randomised when jitter is
///   requested;
/// * otherwise a maximum delay above the delay produces a uniform random policy;
/// * otherwise the policy waits for the delay every time.
///
/// Every delay is expressed in milliseconds and is clamped to at least one.
#[derive(Clone, Default)]
pub struct BackOffPolicyBuilder {
    delay: Option<u64>,
    max_delay: Option<u64>,
    multiplier: Option<f64>,
    random: Option<bool>,
    sleeper: Option<Arc<dyn Sleeper>>,
    delay_supplier: Option<DelaySupplier>,
    max_delay_supplier: Option<DelaySupplier>,
    multiplier_supplier: Option<MultiplierSupplier>,
    random_supplier: Option<RandomSupplier>,
}

impl BackOffPolicyBuilder {
    /// Creates a builder with a one second default delay.
    pub fn new() -> Self {
        Self {
            delay: Some(DEFAULT_BACK_OFF_PERIOD),
            ..Self::default()
        }
    }

    /// Creates the default policy: a fixed wait of one second.
    pub fn new_default_policy() -> BuiltBackOffPolicy {
        Self::new().build()
    }

    /// Sets the canonical delay.
    ///
    /// It is the initial wait of an exponential policy and the minimum of a uniform random
    /// policy.
    pub fn delay(mut self, delay: u64) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Sets the maximum delay.
    ///
    /// For an exponential policy a value at or below the delay falls back to the default
    /// maximum, and for a uniform random policy the value must be above the delay for the
    /// policy to be selected.
    pub fn max_delay(mut self, max_delay: u64) -> Self {
        self.max_delay = Some(max_delay);
        self
    }

    /// Sets the growth factor; a positive value selects an exponential policy.
    pub fn multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = Some(multiplier);
        self
    }

    /// Requests jitter for the exponential policy.
    pub fn random(mut self, random: bool) -> Self {
        self.random = Some(random);
        self
    }

    /// Routes every wait through a custom sleeper.
    pub fn sleeper(mut self, sleeper: Arc<dyn Sleeper>) -> Self {
        self.sleeper = Some(sleeper);
        self
    }

    /// Installs a dynamic source for the delay.
    pub fn delay_supplier<F>(mut self, supplier: F) -> Self
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        self.delay_supplier = Some(Arc::new(supplier));
        self
    }

    /// Installs a dynamic source for the maximum delay.
    pub fn max_delay_supplier<F>(mut self, supplier: F) -> Self
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        self.max_delay_supplier = Some(Arc::new(supplier));
        self
    }

    /// Installs a dynamic source for the multiplier.
    pub fn multiplier_supplier<F>(mut self, supplier: F) -> Self
    where
        F: Fn() -> f64 + Send + Sync + 'static,
    {
        self.multiplier_supplier = Some(Arc::new(supplier));
        self
    }

    /// Installs a dynamic source for the jitter flag.
    pub fn random_supplier<F>(mut self, supplier: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.random_supplier = Some(Arc::new(supplier));
        self
    }

    /// Builds the policy.
    pub fn build(self) -> BuiltBackOffPolicy {
        let multiplier_selected = self
            .multiplier
            .map(|multiplier| multiplier > 0.0)
            .unwrap_or(false)
            || self.multiplier_supplier.is_some();

        if multiplier_selected {
            let mut policy = if self.is_random() {
                let mut policy = ExponentialRandomBackOffPolicy::new();
                apply_exponential_settings(policy.inner_mut(), &self);
                if let Some(sleeper) = self.sleeper {
                    policy.set_sleeper(sleeper);
                }
                return BuiltBackOffPolicy::ExponentialRandom(policy);
            } else {
                ExponentialBackOffPolicy::new()
            };
            apply_exponential_settings(&mut policy, &self);
            if let Some(sleeper) = self.sleeper {
                policy.set_sleeper(sleeper);
            }
            return BuiltBackOffPolicy::Exponential(policy);
        }

        let uniform_selected = match (self.delay, self.max_delay) {
            (Some(delay), Some(max_delay)) => max_delay > delay,
            _ => false,
        };

        if uniform_selected {
            let mut policy = UniformRandomBackOffPolicy::new();
            if let Some(delay) = self.delay {
                policy.set_min_back_off_period(delay);
            }
            if let Some(supplier) = self.delay_supplier {
                policy.set_min_back_off_period_supplier(move || supplier());
            }
            if let Some(max_delay) = self.max_delay {
                policy.set_max_back_off_period(max_delay);
            }
            if let Some(supplier) = self.max_delay_supplier {
                policy.set_max_back_off_period_supplier(move || supplier());
            }
            if let Some(sleeper) = self.sleeper {
                policy.set_sleeper(sleeper);
            }
            return BuiltBackOffPolicy::UniformRandom(policy);
        }

        let mut policy = FixedBackOffPolicy::new();
        if let Some(supplier) = self.delay_supplier {
            policy.set_back_off_period_supplier(move || supplier());
        } else if let Some(delay) = self.delay {
            policy.set_back_off_period(delay);
        }
        if let Some(sleeper) = self.sleeper {
            policy.set_sleeper(sleeper);
        }
        BuiltBackOffPolicy::Fixed(policy)
    }

    fn is_random(&self) -> bool {
        self.random_supplier
            .as_ref()
            .map(|supplier| supplier())
            .unwrap_or(false)
            || self.random.unwrap_or(false)
    }
}

fn apply_exponential_settings(
    policy: &mut ExponentialBackOffPolicy,
    builder: &BackOffPolicyBuilder,
) {
    if let Some(delay) = builder.delay {
        policy.set_initial_interval(delay);
    }
    if let Some(supplier) = builder.delay_supplier.clone() {
        policy.set_initial_interval_supplier(move || supplier());
    }
    if let Some(multiplier) = builder.multiplier {
        policy.set_multiplier(multiplier);
    }
    if let Some(supplier) = builder.multiplier_supplier.clone() {
        policy.set_multiplier_supplier(move || supplier());
    }
    if let (Some(max_delay), Some(delay)) = (builder.max_delay, builder.delay) {
        policy.set_max_interval(if max_delay > delay {
            max_delay
        } else {
            DEFAULT_MAX_INTERVAL
        });
    }
    if let Some(supplier) = builder.max_delay_supplier.clone() {
        policy.set_max_interval_supplier(move || supplier());
    }
}

#[cfg(test)]
mod tests {
    use super::{BackOffPolicyBuilder, BuiltBackOffPolicy};

    #[test]
    fn the_default_policy_waits_one_second() {
        let policy = BackOffPolicyBuilder::new_default_policy();

        match policy {
            BuiltBackOffPolicy::Fixed(policy) => assert_eq!(policy.get_back_off_period(), 1_000),
            other => panic!("expected a fixed policy, got {other}"),
        }
    }

    #[test]
    fn a_multiplier_selects_an_exponential_policy() {
        let policy = BackOffPolicyBuilder::new()
            .delay(10)
            .max_delay(100)
            .multiplier(2.0)
            .build();

        match policy {
            BuiltBackOffPolicy::Exponential(policy) => {
                assert_eq!(policy.get_initial_interval(), 10);
                assert_eq!(policy.get_multiplier(), 2.0);
                assert_eq!(policy.get_max_interval(), 100);
            }
            other => panic!("expected an exponential policy, got {other}"),
        }
    }

    #[test]
    fn a_random_multiplier_selects_a_jittered_policy() {
        let policy = BackOffPolicyBuilder::new()
            .delay(10)
            .max_delay(100)
            .multiplier(2.0)
            .random(true)
            .build();

        assert!(matches!(policy, BuiltBackOffPolicy::ExponentialRandom(_)));
    }

    #[test]
    fn a_maximum_above_the_delay_selects_a_uniform_random_policy() {
        let policy = BackOffPolicyBuilder::new().delay(10).max_delay(20).build();

        match policy {
            BuiltBackOffPolicy::UniformRandom(policy) => {
                assert_eq!(policy.get_min_back_off_period(), 10);
                assert_eq!(policy.get_max_back_off_period(), 20);
            }
            other => panic!("expected a uniform random policy, got {other}"),
        }
    }

    #[test]
    fn a_maximum_below_the_delay_leaves_the_fixed_policy() {
        let policy = BackOffPolicyBuilder::new().delay(20).max_delay(10).build();

        match policy {
            BuiltBackOffPolicy::Fixed(policy) => assert_eq!(policy.get_back_off_period(), 20),
            other => panic!("expected a fixed policy, got {other}"),
        }
    }

    #[test]
    fn an_exponential_maximum_below_the_delay_falls_back_to_the_default() {
        let policy = BackOffPolicyBuilder::new()
            .delay(1_000)
            .max_delay(10)
            .multiplier(2.0)
            .build();

        match policy {
            BuiltBackOffPolicy::Exponential(policy) => {
                assert_eq!(policy.get_max_interval(), super::DEFAULT_MAX_INTERVAL);
            }
            other => panic!("expected an exponential policy, got {other}"),
        }
    }
}
