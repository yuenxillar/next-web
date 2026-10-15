//! Building the configured instances at startup, and registering them.
//!
//! Everything a properties file declares is built once, here, and registered
//! **twice under the same name and behind one `Arc`**: in the process-wide
//! registry that `name = "..."` looks in, and in the application context. Because
//! it is one `Arc`, the two routes cannot hand out different instances.
//!
//! A setting outside its documented range fails the build, which is deliberate:
//! this runs while the application is starting, before any call is guarded, so a
//! configuration mistake is a startup error rather than a surprise later.

use std::{error::Error, sync::Arc, time::Duration};

use next_web_context::ApplicationContextExt;
use next_web_core::{
    ApplicationContext, Ordered, async_trait,
    traits::config::auto_configuration::AutoConfiguration,
};
use next_web_macros::singleton;

use crate::{
    bulkhead::{Bulkhead, BulkheadConfig},
    circuitbreaker::{CircuitBreaker, CircuitBreakerConfig},
    ratelimiter::{RateLimiter, RateLimiterConfig},
    registry::{ResilienceRegistry, global_registry},
    retry::{Retry, RetryConfig},
    sliding_window::SlidingWindowType,
    timelimiter::{TimeLimiter, TimeLimiterConfig},
};

use super::{
    BulkheadInstanceProperties, BulkheadProperties, CircuitBreakerInstanceProperties,
    CircuitBreakerProperties, DurationValue, RateLimiterInstanceProperties, RateLimiterProperties,
    RetryInstanceProperties, RetryProperties, SlidingWindowTypeProperty,
    TimeLimiterInstanceProperties, TimeLimiterProperties,
};

/// Builds the configured instances at startup and registers them.
///
/// Each field is one pattern's settings. The names are not free: the
/// `#[singleton]` machinery derives the lookup name from each *field* name, and
/// it has to match the name the properties macro gave the struct — so
/// `rate_limiter_properties` finds `RateLimiterProperties`, and so on down.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct ResilienceAutoConfiguration {
    /// `next.ratelimiter`
    pub rate_limiter_properties: RateLimiterProperties,
    /// `next.circuitbreaker`
    pub circuit_breaker_properties: CircuitBreakerProperties,
    /// `next.bulkhead`
    pub bulkhead_properties: BulkheadProperties,
    /// `next.timelimiter`
    pub time_limiter_properties: TimeLimiterProperties,
    /// `next.retry`
    pub retry_properties: RetryProperties,
}

impl ResilienceAutoConfiguration {
    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }

    /// Builds every configured instance and registers it in `registry`.
    ///
    /// Returns what was registered so a caller can put the same instances
    /// somewhere else as well — which is what the application context gets.
    ///
    /// # Panics
    ///
    /// When a setting is outside its range, when a duration cannot be read, or
    /// when two instance names in one file would answer to the same name.
    pub fn register_in(&self, registry: &ResilienceRegistry) -> RegisteredInstances {
        let mut registered = RegisteredInstances::default();

        for (name, instance) in &self.rate_limiter_properties.instances {
            let limiter = Arc::new(RateLimiter::new(
                name,
                rate_limiter_config(name, instance),
            ));
            registry.register_rate_limiter(name, Arc::clone(&limiter));
            registered.rate_limiters.push(Registered {
                instance: limiter,
                name: name.clone(),
            });
        }

        for (name, instance) in &self.circuit_breaker_properties.instances {
            let breaker = Arc::new(CircuitBreaker::new(
                name,
                circuit_breaker_config(name, instance),
            ));
            registry.register_circuit_breaker(name, Arc::clone(&breaker));
            registered.circuit_breakers.push(Registered {
                instance: breaker,
                name: name.clone(),
            });
        }

        for (name, instance) in &self.bulkhead_properties.instances {
            let bulkhead = Arc::new(Bulkhead::new(name, bulkhead_config(name, instance)));
            registry.register_bulkhead(name, Arc::clone(&bulkhead));
            registered.bulkheads.push(Registered {
                instance: bulkhead,
                name: name.clone(),
            });
        }

        for (name, instance) in &self.time_limiter_properties.instances {
            let limiter = Arc::new(TimeLimiter::new(
                name,
                time_limiter_config(name, instance),
            ));
            registry.register_time_limiter(name, Arc::clone(&limiter));
            registered.time_limiters.push(Registered {
                instance: limiter,
                name: name.clone(),
            });
        }

        for (name, instance) in &self.retry_properties.instances {
            let retry = Arc::new(Retry::new(name, retry_config(name, instance)));
            registry.register_retry(name, Arc::clone(&retry));
            registered.retries.push(Registered {
                instance: retry,
                name: name.clone(),
            });
        }

        registered
    }
}

#[async_trait]
impl AutoConfiguration for ResilienceAutoConfiguration {
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>> {
        // The registry is what `name = "..."` on the attribute macros looks in;
        // the context is for code that resolves things through the container.
        // Registering one set of instances in both is what keeps them the same.
        let registered = self.register_in(global_registry());

        // A file that declares instances has declared the names, so a name it does
        // not declare is a mistake to report rather than a default to build in
        // silence. Nothing is changed for an application that configures none.
        if !registered.is_empty() {
            global_registry().set_strict_names(true);
        }

        registered.insert_into(ctx);
        Ok(())
    }
}

impl Ordered for ResilienceAutoConfiguration {}

/// One configured instance, and the name it was registered under.
#[derive(Debug, Clone)]
pub struct Registered<T> {
    /// The instance.
    pub instance: Arc<T>,
    /// The name it was registered under: the one the file gave it, unchanged.
    pub name: String,
}

/// Everything one run of the configuration built.
#[derive(Debug, Default)]
pub struct RegisteredInstances {
    /// Rate limiters.
    pub rate_limiters: Vec<Registered<RateLimiter>>,
    /// Circuit breakers.
    pub circuit_breakers: Vec<Registered<CircuitBreaker>>,
    /// Bulkheads.
    pub bulkheads: Vec<Registered<Bulkhead>>,
    /// Time limiters.
    pub time_limiters: Vec<Registered<TimeLimiter>>,
    /// Retries.
    pub retries: Vec<Registered<Retry>>,
}

impl RegisteredInstances {
    /// Whether nothing was registered, which is what an application that
    /// configures no instance produces.
    pub fn is_empty(&self) -> bool {
        self.rate_limiters.is_empty()
            && self.circuit_breakers.is_empty()
            && self.bulkheads.is_empty()
            && self.time_limiters.is_empty()
            && self.retries.is_empty()
    }

    /// Registers every instance in an application context as well, under the same
    /// names and behind the same `Arc` as the registry.
    pub fn insert_into(&self, ctx: &mut dyn ApplicationContext) {
        for entry in &self.rate_limiters {
            insert(ctx, entry);
        }
        for entry in &self.circuit_breakers {
            insert(ctx, entry);
        }
        for entry in &self.bulkheads {
            insert(ctx, entry);
        }
        for entry in &self.time_limiters {
            insert(ctx, entry);
        }
        for entry in &self.retries {
            insert(ctx, entry);
        }
    }
}

/// Puts one instance in the context under its name.
///
/// A free function rather than a method because the instance type has to be
/// nameable for the context's key, and a generic method on the collection would
/// not know which one it was handed.
fn insert<T: Send + Sync + 'static>(ctx: &mut dyn ApplicationContext, entry: &Registered<T>) {
    ctx.insert_singleton_with_name(Arc::clone(&entry.instance), entry.name.clone());
}

/// Reads a duration setting, failing the configuration rather than a call.
fn duration(value: &DurationValue, pattern: &str, instance: &str, setting: &str) -> Duration {
    value.to_duration().unwrap_or_else(|error| {
        panic!("next.{pattern}.instances.{instance}: {setting} {error}")
    })
}

fn rate_limiter_config(instance_name: &str, instance: &RateLimiterInstanceProperties) -> RateLimiterConfig {
    let mut builder = RateLimiterConfig::builder();

    if let Some(value) = instance.limit_for_period {
        builder = builder.limit_for_period(value);
    }
    if let Some(value) = &instance.limit_refresh_period {
        let value = duration(value, "ratelimiter", instance_name, "limitRefreshPeriod");
        builder = builder.limit_refresh_period(value);
    }
    if let Some(value) = &instance.timeout_duration {
        let value = duration(value, "ratelimiter", instance_name, "timeoutDuration");
        builder = builder.timeout_duration(value);
    }

    builder.build()
}

fn circuit_breaker_config(
    instance_name: &str,
    instance: &CircuitBreakerInstanceProperties,
) -> CircuitBreakerConfig {
    let mut builder = CircuitBreakerConfig::builder();
    let read = |value: &DurationValue, setting: &str| {
        duration(value, "circuitbreaker", instance_name, setting)
    };

    if let Some(value) = instance.sliding_window_type {
        builder = builder.sliding_window_type(match value {
            SlidingWindowTypeProperty::CountBased => SlidingWindowType::CountBased,
            SlidingWindowTypeProperty::TimeBased => SlidingWindowType::TimeBased,
        });
    }
    if let Some(value) = instance.sliding_window_size {
        builder = builder.sliding_window_size(value);
    }
    if let Some(value) = &instance.sliding_window_time {
        builder = builder.sliding_window_time(read(value, "slidingWindowTime"));
    }
    if let Some(value) = instance.time_window_bucket_count {
        builder = builder.time_window_bucket_count(value);
    }
    if let Some(value) = instance.minimum_number_of_calls {
        builder = builder.minimum_number_of_calls(value);
    }
    if let Some(value) = instance.failure_rate_threshold {
        builder = builder.failure_rate_threshold(value);
    }
    if let Some(value) = instance.slow_call_rate_threshold {
        builder = builder.slow_call_rate_threshold(value);
    }
    if let Some(value) = &instance.slow_call_duration_threshold {
        builder = builder.slow_call_duration_threshold(read(value, "slowCallDurationThreshold"));
    }
    if let Some(value) = &instance.wait_duration_in_open_state {
        builder = builder.wait_duration_in_open_state(read(value, "waitDurationInOpenState"));
    }
    if let Some(value) = instance.permitted_number_of_calls_in_half_open_state {
        builder = builder.permitted_number_of_calls_in_half_open_state(value);
    }
    if let Some(value) = &instance.max_wait_duration_in_half_open_state {
        builder =
            builder.max_wait_duration_in_half_open_state(read(value, "maxWaitDurationInHalfOpenState"));
    }

    builder.build()
}

fn bulkhead_config(instance_name: &str, instance: &BulkheadInstanceProperties) -> BulkheadConfig {
    let mut builder = BulkheadConfig::builder();

    if let Some(value) = instance.max_concurrent_calls {
        builder = builder.max_concurrent_calls(value);
    }
    if let Some(value) = &instance.max_wait_duration {
        let value = duration(value, "bulkhead", instance_name, "maxWaitDuration");
        builder = builder.max_wait_duration(value);
    }

    builder.build()
}

fn time_limiter_config(
    instance_name: &str,
    instance: &TimeLimiterInstanceProperties,
) -> TimeLimiterConfig {
    let mut builder = TimeLimiterConfig::builder();

    if let Some(value) = &instance.timeout_duration {
        let value = duration(value, "timelimiter", instance_name, "timeoutDuration");
        builder = builder.timeout_duration(value);
    }
    if let Some(value) = instance.cancel_running_future {
        builder = builder.cancel_running_future(value);
    }

    builder.build()
}

fn retry_config(instance_name: &str, instance: &RetryInstanceProperties) -> RetryConfig {
    let mut builder = RetryConfig::builder();

    if let Some(value) = instance.max_attempts {
        builder = builder.max_attempts(value);
    }
    if let Some(value) = &instance.wait_duration {
        let value = duration(value, "retry", instance_name, "waitDuration");
        builder = builder.wait_duration(value);
    }
    if let Some(value) = instance.fail_after_max_attempts {
        builder = builder.fail_after_max_attempts(value);
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use next_web_core::util::indexmap::IndexMap;

    use super::ResilienceAutoConfiguration;
    use crate::{
        autoconfigure::{
            BulkheadProperties, CircuitBreakerProperties, DurationValue, RateLimiterProperties,
            RetryProperties, TimeLimiterProperties,
        },
        registry::ResilienceRegistry,
    };

    fn properties<T: serde::de::DeserializeOwned>(entries: &[(&str, &str)], prefix: &str) -> T {
        let mut environment = next_web_core::env::BaseEnvironment::new();
        {
            use next_web_core::env::ConfigurableEnvironment;
            environment
                .property_sources()
                .add_last(Box::new(next_web_core::env::MapPropertySource::new(
                    "test".to_owned(),
                    entries
                        .iter()
                        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                        .collect::<IndexMap<_, _>>(),
                )));
        }

        next_web_core::env::Binder::new(&environment, prefix)
            .bind::<T>()
            .unwrap_or_else(|error| panic!("{prefix} should bind: {error}"))
    }

    /// A configuration with one instance of each pattern, as a file would write it.
    fn configuration() -> ResilienceAutoConfiguration {
        ResilienceAutoConfiguration {
            rate_limiter_properties: properties(
                &[
                    (
                        "next.ratelimiter.instances.backendA.limitForPeriod",
                        "5",
                    ),
                    (
                        "next.ratelimiter.instances.backendA.limitRefreshPeriod",
                        "1s",
                    ),
                    (
                        "next.ratelimiter.instances.backendA.timeoutDuration",
                        "0",
                    ),
                ],
                "next.ratelimiter",
            ),
            circuit_breaker_properties: properties(
                &[
                    (
                        "next.circuitbreaker.instances.backendB.slidingWindowSize",
                        "10",
                    ),
                    (
                        "next.circuitbreaker.instances.backendB.failureRateThreshold",
                        "50",
                    ),
                    (
                        "next.circuitbreaker.instances.backendB.waitDurationInOpenState",
                        "10s",
                    ),
                    (
                        "next.circuitbreaker.instances.backendB.permittedNumberOfCallsInHalfOpenState",
                        "3",
                    ),
                ],
                "next.circuitbreaker",
            ),
            bulkhead_properties: properties(
                &[
                    (
                        "next.bulkhead.instances.backendD.maxConcurrentCalls",
                        "10",
                    ),
                    ("next.bulkhead.instances.backendD.maxWaitDuration", "10ms"),
                ],
                "next.bulkhead",
            ),
            time_limiter_properties: properties(
                &[
                    (
                        "next.timelimiter.instances.backendE.timeoutDuration",
                        "2s",
                    ),
                    (
                        "next.timelimiter.instances.backendE.cancelRunningFuture",
                        "true",
                    ),
                ],
                "next.timelimiter",
            ),
            retry_properties: properties(
                &[
                    ("next.retry.instances.backendF.maxAttempts", "4"),
                    ("next.retry.instances.backendF.waitDuration", "500ms"),
                ],
                "next.retry",
            ),
        }
    }

    #[test]
    fn what_a_file_says_is_what_the_instance_is_built_with() {
        let registry = ResilienceRegistry::new();
        configuration().register_in(&registry);

        let limiter = registry
            .find_rate_limiter("backendA")
            .expect("the limiter is registered");
        assert_eq!(limiter.config().limit_for_period(), 5);
        assert_eq!(limiter.config().limit_refresh_period(), Duration::from_secs(1));
        assert_eq!(limiter.config().timeout_duration(), Duration::ZERO);

        let breaker = registry
            .find_circuit_breaker("backendB")
            .expect("the breaker is registered");
        assert_eq!(breaker.config().sliding_window_size(), 10);
        assert_eq!(breaker.config().failure_rate_threshold(), 50.0);
        assert_eq!(
            breaker.config().wait_duration_in_open_state(),
            Duration::from_secs(10)
        );
        assert_eq!(
            breaker.config().permitted_number_of_calls_in_half_open_state(),
            3
        );

        let bulkhead = registry
            .find_bulkhead("backendD")
            .expect("the bulkhead is registered");
        assert_eq!(bulkhead.config().max_concurrent_calls(), 10);
        assert_eq!(
            bulkhead.config().max_wait_duration(),
            Duration::from_millis(10)
        );

        let limiter = registry
            .find_time_limiter("backendE")
            .expect("the time limiter is registered");
        assert_eq!(
            limiter.config().timeout_duration(),
            Duration::from_secs(2)
        );
        assert!(limiter.config().cancel_running_future());

        let retry = registry.find_retry("backendF").expect("the retry is registered");
        assert_eq!(retry.config().max_attempts(), 4);
        assert_eq!(retry.config().wait_duration(), Duration::from_millis(500));
    }

    #[test]
    fn an_instance_is_registered_under_the_name_the_file_gave_it() {
        let registry = ResilienceRegistry::new();
        let registered = configuration().register_in(&registry);

        let entry = registered
            .rate_limiters
            .first()
            .expect("one limiter was built");
        assert_eq!(entry.name, "backendA");

        assert!(Arc::ptr_eq(
            &registry.find_rate_limiter("backendA").expect("registered"),
            &entry.instance
        ));

        // The name is used exactly as written. Another spelling is another name,
        // and does not reach this instance.
        assert!(registry.find_rate_limiter("backend_a").is_none());
    }

    #[test]
    fn a_setting_outside_its_range_fails_the_configuration_not_a_call() {
        let properties: RateLimiterProperties = properties(
            &[("next.ratelimiter.instances.zero.limitForPeriod", "0")],
            "next.ratelimiter",
        );
        let configuration = ResilienceAutoConfiguration {
            rate_limiter_properties: properties,
            ..configuration()
        };

        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            configuration.register_in(&ResilienceRegistry::new())
        }));
        assert!(failed.is_err(), "a limit of zero should not build");
    }

    #[test]
    fn an_unreadable_duration_names_the_instance_and_the_setting() {
        let properties: RateLimiterProperties = properties(
            &[(
                "next.ratelimiter.instances.backendA.limitRefreshPeriod",
                "often",
            )],
            "next.ratelimiter",
        );
        let configuration = ResilienceAutoConfiguration {
            rate_limiter_properties: properties,
            ..configuration()
        };

        let message = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            configuration.register_in(&ResilienceRegistry::new())
        }))
        .map_err(|payload| {
            payload
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| "<not a string>".to_owned())
        })
        .expect_err("it should not build");

        assert!(message.contains("backendA"), "{message}");
        assert!(message.contains("limitRefreshPeriod"), "{message}");
        assert!(message.contains("often"), "{message}");
    }

    #[test]
    fn a_configuration_that_declares_nothing_registers_nothing() {
        let registry = ResilienceRegistry::new();
        let configuration = ResilienceAutoConfiguration {
            rate_limiter_properties: RateLimiterProperties {
                instances: HashMap::new(),
            },
            circuit_breaker_properties: CircuitBreakerProperties::default(),
            bulkhead_properties: BulkheadProperties::default(),
            time_limiter_properties: TimeLimiterProperties::default(),
            retry_properties: RetryProperties::default(),
        };

        let registered = configuration.register_in(&registry);
        assert!(registered.rate_limiters.is_empty());
        assert!(registered.circuit_breakers.is_empty());
        assert!(registry.is_empty());
    }

    #[test]
    fn a_name_the_file_does_not_declare_is_reported_rather_than_defaulted() {
        let registry = ResilienceRegistry::new();

        // Off until a file declares instances, so an application that configures
        // none keeps resolving whatever names it likes.
        assert!(!registry.is_strict_names());
        assert_eq!(
            registry
                .rate_limiter_or_default("anything")
                .config()
                .limit_for_period(),
            50,
            "with nothing configured, an unknown name is still given the defaults"
        );

        configuration().register_in(&registry);
        registry.set_strict_names(true);

        // A declared name resolves to what the file said.
        assert_eq!(
            registry
                .rate_limiter_or_default("backendA")
                .config()
                .limit_for_period(),
            5
        );

        // One the file does not declare is reported and then runs on the defaults,
        // rather than taking the endpoint down. The report is a log line, so what
        // is asserted here is the behaviour that log accompanies.
        let undeclared = registry.rate_limiter_or_default("backend_a");
        assert_eq!(
            undeclared.config().limit_for_period(),
            50,
            "an undeclared name is reported and served on the defaults, not refused"
        );
    }

    #[test]
    fn a_duration_setting_reaches_the_instance_as_a_duration() {
        // The text form and the bare number have to mean the same thing wherever
        // they are used.
        let properties: BulkheadProperties = properties(
            &[("next.bulkhead.instances.b.maxWaitDuration", "250ms")],
            "next.bulkhead",
        );
        assert_eq!(
            properties.instances["b"].max_wait_duration,
            Some(DurationValue::Text("250ms".to_owned()))
        );
    }
}
