//! The settings each pattern reads from the application's properties.
//!
//! One struct per pattern, each with its own prefix, and one struct per instance
//! holding that instance's settings. Every field is optional: what a file leaves
//! out keeps the builder's default, which is what makes a half-written instance
//! useful.
//!
//! Field names are `snake_case` in Rust and accept the `camelCase` spelling in
//! the file, so a configuration copied from a Resilience4j example works as
//! written. A key inside an instance that matches no field is an error rather
//! than a setting that quietly does nothing.

use std::collections::HashMap;

use super::DurationValue;

/// Rate limiter instances, under `next.ratelimiter`.
#[next_web_macros::configuration_properties(prefix = "next.ratelimiter")]
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct RateLimiterProperties {
    /// The configured instances, keyed by the name that reaches them.
    #[serde(default)]
    pub instances: HashMap<String, RateLimiterInstanceProperties>,
}

/// One rate limiter's settings.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RateLimiterInstanceProperties {
    /// How many permissions each refresh period grants. Default: 50.
    #[serde(alias = "limitForPeriod")]
    pub limit_for_period: Option<u32>,
    /// How long a granted permission lasts. Default: 500ms.
    #[serde(alias = "limitRefreshPeriod")]
    pub limit_refresh_period: Option<DurationValue>,
    /// How long a caller waits for a permission before being refused. Default:
    /// 5s; `0` refuses at once rather than waiting.
    #[serde(alias = "timeoutDuration")]
    pub timeout_duration: Option<DurationValue>,
}

/// Circuit breaker instances, under `next.circuitbreaker`.
#[next_web_macros::configuration_properties(prefix = "next.circuitbreaker")]
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct CircuitBreakerProperties {
    /// The configured instances, keyed by the name that reaches them.
    #[serde(default)]
    pub instances: HashMap<String, CircuitBreakerInstanceProperties>,
}

/// Which calls the sliding window counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum SlidingWindowTypeProperty {
    /// The last *N* recorded calls.
    #[serde(alias = "COUNT_BASED", alias = "countBased", alias = "count_based")]
    CountBased,
    /// The calls recorded during the last *T*.
    #[serde(alias = "TIME_BASED", alias = "timeBased", alias = "time_based")]
    TimeBased,
}

/// One circuit breaker's settings.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CircuitBreakerInstanceProperties {
    /// How the window aggregates calls. Default: `CountBased`.
    #[serde(alias = "slidingWindowType")]
    pub sliding_window_type: Option<SlidingWindowTypeProperty>,
    /// How many calls the count-based window holds. Default: 100.
    #[serde(alias = "slidingWindowSize")]
    pub sliding_window_size: Option<u32>,
    /// How many calls the time-based window covers. Default: 60s.
    #[serde(alias = "slidingWindowTime")]
    pub sliding_window_time: Option<DurationValue>,
    /// How many buckets the time-based window is divided into. Default: 10.
    #[serde(alias = "timeWindowBucketCount")]
    pub time_window_bucket_count: Option<u32>,
    /// Calls needed in the window before a rate is judged at all. Default: 100.
    #[serde(alias = "minimumNumberOfCalls")]
    pub minimum_number_of_calls: Option<u32>,
    /// Failure rate, as a percentage, at or above which the breaker opens.
    /// Default: 50.
    #[serde(alias = "failureRateThreshold")]
    pub failure_rate_threshold: Option<f32>,
    /// Slow call rate, as a percentage, at or above which the breaker opens.
    /// Default: 100.
    #[serde(alias = "slowCallRateThreshold")]
    pub slow_call_rate_threshold: Option<f32>,
    /// How long a call may take before it counts as slow. Default: 60s.
    #[serde(alias = "slowCallDurationThreshold")]
    pub slow_call_duration_threshold: Option<DurationValue>,
    /// How long the breaker stays open before letting calls through again.
    /// Default: 60s.
    #[serde(alias = "waitDurationInOpenState")]
    pub wait_duration_in_open_state: Option<DurationValue>,
    /// How many calls are permitted once half-open. Default: 10.
    #[serde(alias = "permittedNumberOfCallsInHalfOpenState")]
    pub permitted_number_of_calls_in_half_open_state: Option<u32>,
    /// How long a permitted call may take in the half-open state. Default: 60s.
    #[serde(alias = "maxWaitDurationInHalfOpenState")]
    pub max_wait_duration_in_half_open_state: Option<DurationValue>,
}

/// Bulkhead instances, under `next.bulkhead`.
#[next_web_macros::configuration_properties(prefix = "next.bulkhead")]
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct BulkheadProperties {
    /// The configured instances, keyed by the name that reaches them.
    #[serde(default)]
    pub instances: HashMap<String, BulkheadInstanceProperties>,
}

/// One bulkhead's settings.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BulkheadInstanceProperties {
    /// How many calls may run at once. Default: 25.
    #[serde(alias = "maxConcurrentCalls")]
    pub max_concurrent_calls: Option<u32>,
    /// How long a caller waits for a free slot before being refused. Default:
    /// `0`, which refuses at once.
    #[serde(alias = "maxWaitDuration")]
    pub max_wait_duration: Option<DurationValue>,
}

/// Time limiter instances, under `next.timelimiter`.
#[next_web_macros::configuration_properties(prefix = "next.timelimiter")]
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct TimeLimiterProperties {
    /// The configured instances, keyed by the name that reaches them.
    #[serde(default)]
    pub instances: HashMap<String, TimeLimiterInstanceProperties>,
}

/// One time limiter's settings.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TimeLimiterInstanceProperties {
    /// How long a call may take before it is abandoned. Default: 60s.
    #[serde(alias = "timeoutDuration")]
    pub timeout_duration: Option<DurationValue>,
    /// Whether the abandoned future is cancelled rather than left running.
    /// Default: `true`.
    #[serde(alias = "cancelRunningFuture")]
    pub cancel_running_future: Option<bool>,
}

/// Retry instances, under `next.retry`.
#[next_web_macros::configuration_properties(prefix = "next.retry")]
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct RetryProperties {
    /// The configured instances, keyed by the name that reaches them.
    #[serde(default)]
    pub instances: HashMap<String, RetryInstanceProperties>,
}

/// One retry's settings.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetryInstanceProperties {
    /// How many attempts a call gets, the first included. Default: 3.
    #[serde(alias = "maxAttempts")]
    pub max_attempts: Option<u32>,
    /// How long to wait between attempts, before any backoff. Default: 500ms.
    #[serde(alias = "waitDuration")]
    pub wait_duration: Option<DurationValue>,
    /// Whether a spent budget fails the call instead of returning the last
    /// error. Default: `false`, which reports the last failure.
    #[serde(alias = "failAfterMaxAttempts")]
    pub fail_after_max_attempts: Option<bool>,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use next_web_core::{
        env::{BaseEnvironment, Binder, ConfigurableEnvironment, MapPropertySource},
        util::indexmap::IndexMap,
    };

    use super::{
        BulkheadProperties, CircuitBreakerInstanceProperties, CircuitBreakerProperties,
        RateLimiterInstanceProperties, RateLimiterProperties, RetryProperties,
        SlidingWindowTypeProperty, TimeLimiterProperties,
    };
    use crate::autoconfigure::DurationValue;

    /// An environment holding exactly the given properties, the way the
    /// framework hands the application's file to the binder.
    fn environment(properties: &[(&str, &str)]) -> BaseEnvironment {
        let mut environment = BaseEnvironment::new();
        environment
            .property_sources()
            .add_last(Box::new(MapPropertySource::new(
                "test".to_owned(),
                properties
                    .iter()
                    .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                    .collect::<IndexMap<_, _>>(),
            )));
        environment
    }

    /// Binds a prefix into a properties struct.
    fn bind<T: serde::de::DeserializeOwned>(properties: &[(&str, &str)], prefix: &str) -> T {
        Binder::new(&environment(properties), prefix)
            .bind::<T>()
            .unwrap_or_else(|error| panic!("{prefix} should bind: {error}"))
    }

    fn duration(value: &DurationValue) -> Duration {
        value
            .to_duration()
            .unwrap_or_else(|error| panic!("{value} should parse: {error}"))
    }

    #[test]
    fn an_instance_binds_field_by_field() {
        let properties: RateLimiterProperties = bind(
            &[
                ("next.ratelimiter.instances.backendA.limit_for_period", "5"),
                (
                    "next.ratelimiter.instances.backendA.limit_refresh_period",
                    "1s",
                ),
                (
                    "next.ratelimiter.instances.backendA.timeout_duration",
                    "0",
                ),
            ],
            "next.ratelimiter",
        );

        let instance = properties
            .instances
            .get("backendA")
            .expect("the instance is bound");
        assert_eq!(instance.limit_for_period, Some(5));
        assert_eq!(
            duration(instance.limit_refresh_period.as_ref().expect("a period")),
            Duration::from_secs(1)
        );
        assert_eq!(
            duration(instance.timeout_duration.as_ref().expect("a timeout")),
            Duration::ZERO
        );
    }

    /// The point of the aliases: a configuration copied from a Resilience4j
    /// example, and the same configuration written for Rust, are the same
    /// settings.
    #[test]
    fn the_camel_case_spelling_binds_identically_to_the_snake_case_one() {
        let camel: BulkheadProperties = bind(
            &[
                (
                    "next.bulkhead.instances.backendD.maxConcurrentCalls",
                    "10",
                ),
                ("next.bulkhead.instances.backendD.maxWaitDuration", "10ms"),
            ],
            "next.bulkhead",
        );

        let snake: BulkheadProperties = bind(
            &[
                (
                    "next.bulkhead.instances.backendD.max_concurrent_calls",
                    "10",
                ),
                (
                    "next.bulkhead.instances.backendD.max_wait_duration",
                    "10ms",
                ),
            ],
            "next.bulkhead",
        );

        assert_eq!(camel, snake);
        let instance = camel.instances.get("backendD").expect("the instance");
        assert_eq!(instance.max_concurrent_calls, Some(10));
        assert_eq!(
            duration(instance.max_wait_duration.as_ref().expect("a wait")),
            Duration::from_millis(10)
        );
    }

    #[test]
    fn every_duration_form_a_file_may_use_is_read() {
        let properties: TimeLimiterProperties = bind(
            &[
                (
                    "next.timelimiter.instances.zero.timeoutDuration",
                    "0",
                ),
                (
                    "next.timelimiter.instances.millis.timeoutDuration",
                    "250ms",
                ),
                (
                    "next.timelimiter.instances.seconds.timeoutDuration",
                    "2s",
                ),
                (
                    "next.timelimiter.instances.minutes.timeoutDuration",
                    "5m",
                ),
            ],
            "next.timelimiter",
        );

        let at = |name: &str| {
            duration(
                properties.instances[name]
                    .timeout_duration
                    .as_ref()
                    .expect("a timeout"),
            )
        };

        assert_eq!(at("zero"), Duration::ZERO);
        assert_eq!(at("millis"), Duration::from_millis(250));
        assert_eq!(at("seconds"), Duration::from_secs(2));
        assert_eq!(at("minutes"), Duration::from_secs(300));
    }

    #[test]
    fn every_circuit_breaker_setting_binds() {
        let properties: CircuitBreakerProperties = bind(
            &[
                (
                    "next.circuitbreaker.instances.backendB.slidingWindowType",
                    "TIME_BASED",
                ),
                (
                    "next.circuitbreaker.instances.backendB.slidingWindowSize",
                    "10",
                ),
                (
                    "next.circuitbreaker.instances.backendB.slidingWindowTime",
                    "30s",
                ),
                (
                    "next.circuitbreaker.instances.backendB.timeWindowBucketCount",
                    "6",
                ),
                (
                    "next.circuitbreaker.instances.backendB.minimumNumberOfCalls",
                    "5",
                ),
                (
                    "next.circuitbreaker.instances.backendB.failureRateThreshold",
                    "50",
                ),
                (
                    "next.circuitbreaker.instances.backendB.slowCallRateThreshold",
                    "80",
                ),
                (
                    "next.circuitbreaker.instances.backendB.slowCallDurationThreshold",
                    "2s",
                ),
                (
                    "next.circuitbreaker.instances.backendB.waitDurationInOpenState",
                    "10s",
                ),
                (
                    "next.circuitbreaker.instances.backendB.permittedNumberOfCallsInHalfOpenState",
                    "3",
                ),
                (
                    "next.circuitbreaker.instances.backendB.maxWaitDurationInHalfOpenState",
                    "1s",
                ),
            ],
            "next.circuitbreaker",
        );

        let instance = properties
            .instances
            .get("backendB")
            .expect("the instance is bound");
        assert_eq!(
            instance.sliding_window_type,
            Some(SlidingWindowTypeProperty::TimeBased)
        );
        assert_eq!(instance.sliding_window_size, Some(10));
        assert_eq!(instance.time_window_bucket_count, Some(6));
        assert_eq!(instance.minimum_number_of_calls, Some(5));
        assert_eq!(instance.failure_rate_threshold, Some(50.0));
        assert_eq!(instance.slow_call_rate_threshold, Some(80.0));
        assert_eq!(
            instance.permitted_number_of_calls_in_half_open_state,
            Some(3)
        );
        assert_eq!(
            duration(instance.wait_duration_in_open_state.as_ref().unwrap()),
            Duration::from_secs(10)
        );
        assert_eq!(
            duration(instance.sliding_window_time.as_ref().unwrap()),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn the_sliding_window_type_takes_either_spelling() {
        for spelling in ["COUNT_BASED", "countBased", "count_based"] {
            let properties: CircuitBreakerProperties = bind(
                &[(
                    "next.circuitbreaker.instances.b.slidingWindowType",
                    spelling,
                )],
                "next.circuitbreaker",
            );
            assert_eq!(
                properties.instances["b"].sliding_window_type,
                Some(SlidingWindowTypeProperty::CountBased),
                "{spelling} should be read as the count-based window"
            );
        }
    }

    #[test]
    fn every_retry_setting_binds() {
        let properties: RetryProperties = bind(
            &[
                ("next.retry.instances.backendF.maxAttempts", "4"),
                ("next.retry.instances.backendF.waitDuration", "500ms"),
                (
                    "next.retry.instances.backendF.failAfterMaxAttempts",
                    "true",
                ),
            ],
            "next.retry",
        );

        let instance = properties.instances.get("backendF").expect("the instance");
        assert_eq!(instance.max_attempts, Some(4));
        assert_eq!(instance.fail_after_max_attempts, Some(true));
        assert_eq!(
            duration(instance.wait_duration.as_ref().expect("a wait")),
            Duration::from_millis(500)
        );
    }

    /// Inside an instance the binder does not relax names and serde would ignore
    /// a key it does not know, so a typo would otherwise be a setting that does
    /// nothing at all.
    #[test]
    fn a_key_that_matches_no_setting_is_an_error() {
        let error = Binder::new(&environment(&[(
            "next.ratelimiter.instances.backendA.limitForPeriods",
            "5",
        )]), "next.ratelimiter")
        .bind::<RateLimiterProperties>()
        .expect_err("a mistyped key should be reported");

        let message = error.to_string();
        assert!(
            message.contains("limitForPeriods"),
            "the message should name the key: {message}"
        );
    }

    /// An alias **adds** a spelling; it never replaces the one the struct
    /// declares. So every setting can be written the way the Rust field is, and
    /// the two ways have to produce exactly the same settings.
    ///
    /// This is what keeps a project free to write either style, rather than being
    /// forced into camelCase by an alias meant to help it.
    #[test]
    fn the_rust_spelling_binds_the_same_settings_as_the_camel_case_one() {
        let camel: RetryProperties = bind(
            &[
                ("next.retry.instances.backendF.maxAttempts", "4"),
                ("next.retry.instances.backendF.waitDuration", "500ms"),
                (
                    "next.retry.instances.backendF.failAfterMaxAttempts",
                    "true",
                ),
            ],
            "next.retry",
        );
        let snake: RetryProperties = bind(
            &[
                ("next.retry.instances.backendF.max_attempts", "4"),
                ("next.retry.instances.backendF.wait_duration", "500ms"),
                (
                    "next.retry.instances.backendF.fail_after_max_attempts",
                    "true",
                ),
            ],
            "next.retry",
        );
        assert_eq!(
            snake, camel,
            "every retry setting binds under both spellings"
        );

        let camel: TimeLimiterProperties = bind(
            &[
                ("next.timelimiter.instances.e.timeoutDuration", "2s"),
                (
                    "next.timelimiter.instances.e.cancelRunningFuture",
                    "false",
                ),
            ],
            "next.timelimiter",
        );
        let snake: TimeLimiterProperties = bind(
            &[
                ("next.timelimiter.instances.e.timeout_duration", "2s"),
                (
                    "next.timelimiter.instances.e.cancel_running_future",
                    "false",
                ),
            ],
            "next.timelimiter",
        );
        assert_eq!(
            snake, camel,
            "every time limiter setting binds under both spellings"
        );

        let camel: CircuitBreakerProperties = bind(
            &[
                (
                    "next.circuitbreaker.instances.b.slidingWindowType",
                    "TIME_BASED",
                ),
                ("next.circuitbreaker.instances.b.slidingWindowSize", "10"),
                ("next.circuitbreaker.instances.b.slidingWindowTime", "30s"),
                (
                    "next.circuitbreaker.instances.b.timeWindowBucketCount",
                    "6",
                ),
                ("next.circuitbreaker.instances.b.minimumNumberOfCalls", "5"),
                (
                    "next.circuitbreaker.instances.b.failureRateThreshold",
                    "50",
                ),
                (
                    "next.circuitbreaker.instances.b.slowCallRateThreshold",
                    "80",
                ),
                (
                    "next.circuitbreaker.instances.b.slowCallDurationThreshold",
                    "2s",
                ),
                (
                    "next.circuitbreaker.instances.b.waitDurationInOpenState",
                    "10s",
                ),
                (
                    "next.circuitbreaker.instances.b.permittedNumberOfCallsInHalfOpenState",
                    "3",
                ),
                (
                    "next.circuitbreaker.instances.b.maxWaitDurationInHalfOpenState",
                    "1s",
                ),
            ],
            "next.circuitbreaker",
        );
        let snake: CircuitBreakerProperties = bind(
            &[
                (
                    "next.circuitbreaker.instances.b.sliding_window_type",
                    "TIME_BASED",
                ),
                ("next.circuitbreaker.instances.b.sliding_window_size", "10"),
                ("next.circuitbreaker.instances.b.sliding_window_time", "30s"),
                (
                    "next.circuitbreaker.instances.b.time_window_bucket_count",
                    "6",
                ),
                (
                    "next.circuitbreaker.instances.b.minimum_number_of_calls",
                    "5",
                ),
                (
                    "next.circuitbreaker.instances.b.failure_rate_threshold",
                    "50",
                ),
                (
                    "next.circuitbreaker.instances.b.slow_call_rate_threshold",
                    "80",
                ),
                (
                    "next.circuitbreaker.instances.b.slow_call_duration_threshold",
                    "2s",
                ),
                (
                    "next.circuitbreaker.instances.b.wait_duration_in_open_state",
                    "10s",
                ),
                (
                    "next.circuitbreaker.instances.b.permitted_number_of_calls_in_half_open_state",
                    "3",
                ),
                (
                    "next.circuitbreaker.instances.b.max_wait_duration_in_half_open_state",
                    "1s",
                ),
            ],
            "next.circuitbreaker",
        );
        assert_eq!(
            snake, camel,
            "every circuit breaker setting binds under both spellings"
        );

        // The comparison is only meaningful if the bindings are not empty.
        assert_eq!(snake.instances["b"].sliding_window_size, Some(10));
        assert_eq!(snake.instances["b"].minimum_number_of_calls, Some(5));
    }

    #[test]
    fn a_file_that_configures_nothing_binds_no_instances() {
        let properties: RateLimiterProperties = bind(&[("next.application.name", "demo")], "next.ratelimiter");
        assert!(properties.instances.is_empty());

        let properties: CircuitBreakerProperties = bind(&[], "next.circuitbreaker");
        assert!(properties.instances.is_empty());
    }

    #[test]
    fn an_instance_may_set_only_some_of_its_settings() {
        let properties: RateLimiterProperties = bind(
            &[("next.ratelimiter.instances.backendA.limitForPeriod", "5")],
            "next.ratelimiter",
        );

        let instance = properties.instances.get("backendA").expect("the instance");
        assert_eq!(instance.limit_for_period, Some(5));
        assert_eq!(instance.limit_refresh_period, None);
        assert_eq!(instance.timeout_duration, None);
    }

    #[test]
    fn instances_are_keyed_by_the_name_in_the_file_however_it_is_written() {
        let properties: RateLimiterProperties = bind(
            &[
                (
                    "next.ratelimiter.instances.backendE.limitForPeriod",
                    "5",
                ),
                (
                    "next.ratelimiter.instances.backend_a.limitForPeriod",
                    "6",
                ),
            ],
            "next.ratelimiter",
        );

        assert_eq!(
            properties.instances["backendE"].limit_for_period,
            Some(5),
            "a name is kept as written"
        );
        assert_eq!(properties.instances["backend_a"].limit_for_period, Some(6));
    }

    #[test]
    fn an_unreadable_duration_is_reported_with_the_text_that_was_written() {
        let properties: RateLimiterProperties = bind(
            &[(
                "next.ratelimiter.instances.backendA.limitRefreshPeriod",
                "one second",
            )],
            "next.ratelimiter",
        );

        let value = properties.instances["backendA"]
            .limit_refresh_period
            .as_ref()
            .expect("the value binds; it is read later");
        let error = value.to_duration().expect_err("it should not parse");
        assert_eq!(error.value(), "one second");
    }

    #[test]
    fn the_instance_struct_is_reachable_for_a_hand_built_configuration() {
        // Constructing one in code is a supported way to build a test fixture.
        let instance = RateLimiterInstanceProperties {
            limit_for_period: Some(3),
            ..Default::default()
        };
        assert_eq!(instance.limit_for_period, Some(3));

        let _ = CircuitBreakerInstanceProperties::default();
    }
}
