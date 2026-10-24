//! The fluent builder of a retry template.

use std::{sync::Arc, time::Duration};

use next_web_core::any::AnyError;

use crate::{
    Predicate,
    backoff::{
        back_off_policy::BackOffPolicy, exponential_back_off_policy::ExponentialBackOffPolicy,
        exponential_random_back_off_policy::ExponentialRandomBackOffPolicy,
        fixed_back_off_policy::FixedBackOffPolicy, no_back_off_policy::NoBackOffPolicy,
        uniform_random_back_off_policy::UniformRandomBackOffPolicy,
    },
    classifier::{
        binary_error_classifier_builder::BinaryErrorClassifierBuilder,
        error_type_classifier::ErrorTypeClassifier,
    },
    error::retry_error::RetryError,
    policy::{
        always_retry_policy::AlwaysRetryPolicy,
        binary_error_classifier_retry_policy::BinaryErrorClassifierRetryPolicy,
        composite_retry_policy::CompositeRetryPolicy,
        max_attempts_retry_policy::MaxAttemptsRetryPolicy,
        predicate_retry_policy::PredicateRetryPolicy, timeout_retry_policy::TimeoutRetryPolicy,
    },
    retry_listener::RetryListener,
    retry_policy::RetryPolicy,
    support::retry_template::RetryTemplate,
};

/// Builds a [`RetryTemplate`] with a fluent API.
///
/// The defaults are the documented behaviour of a template: a
/// [`MaxAttemptsRetryPolicy`] that allows three attempts, a classifier that retries every
/// error without inspecting causes, and a [`NoBackOffPolicy`].
///
/// The resulting template always uses a [`CompositeRetryPolicy`] that combines the selected
/// attempt policy with a classifier policy, so the attempt limit and the error filter apply
/// together.
///
/// The builder is not thread safe. Configuration mistakes are rejected immediately, because
/// they are startup errors and must not be discovered while serving traffic.
#[derive(Default)]
pub struct RetryTemplateBuilder {
    base_retry_policy: Option<Arc<dyn RetryPolicy>>,
    back_off_policy: Option<Arc<dyn BackOffPolicy>>,
    listeners: Vec<Arc<dyn RetryListener>>,
    classifier_builder: Option<BinaryErrorClassifierBuilder>,
    retry_on_predicate: Option<Arc<dyn Predicate<RetryError>>>,
    error_type_classifier: Option<ErrorTypeClassifier>,
    error_type_is_white_list: Option<bool>,
    error_type_traverse_causes: bool,
    throw_last_exception_on_exhausted: bool,
}

impl RetryTemplateBuilder {
    /// Creates a builder with the default behaviour.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempts an action no more than `max_attempts` times.
    ///
    /// A value of three means one initial attempt and two retries.
    ///
    /// # Panics
    ///
    /// Panics when `max_attempts` is zero or when another retry policy was already selected.
    pub fn max_attempts(mut self, max_attempts: u16) -> Self {
        assert!(max_attempts > 0, "Number of attempts should be positive");
        assert!(
            self.base_retry_policy.is_none(),
            "You have already selected another retry policy"
        );
        self.base_retry_policy = Some(Arc::new(MaxAttemptsRetryPolicy::new(max_attempts)));
        self
    }

    /// Retries until `timeout_millis` have passed since the first attempt.
    ///
    /// # Panics
    ///
    /// Panics when the timeout is zero or when another retry policy was already selected.
    pub fn with_timeout(mut self, timeout_millis: u64) -> Self {
        assert!(
            timeout_millis > 0,
            "timeout_millis should be greater than 0"
        );
        assert!(
            self.base_retry_policy.is_none(),
            "You have already selected another retry policy"
        );
        self.base_retry_policy = Some(Arc::new(TimeoutRetryPolicy::new(timeout_millis)));
        self
    }

    /// Retries until `timeout` has passed since the first attempt.
    ///
    /// # Panics
    ///
    /// Panics when the timeout rounds down to zero milliseconds.
    pub fn with_timeout_from_duration(self, timeout: Duration) -> Self {
        let millis = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX);
        self.with_timeout(millis)
    }

    /// Retries until the action succeeds.
    ///
    /// # Panics
    ///
    /// Panics when another retry policy was already selected.
    pub fn infinite_retry(mut self) -> Self {
        assert!(
            self.base_retry_policy.is_none(),
            "You have already selected another retry policy"
        );
        self.base_retry_policy = Some(Arc::new(AlwaysRetryPolicy::new()));
        self
    }

    /// Uses a custom retry policy.
    ///
    /// # Panics
    ///
    /// Panics when another retry policy was already selected.
    pub fn custom_policy<T>(mut self, policy: T) -> Self
    where
        T: RetryPolicy + 'static,
    {
        assert!(
            self.base_retry_policy.is_none(),
            "You have already selected another retry policy"
        );
        self.base_retry_policy = Some(Arc::new(policy));
        self
    }

    /// Uses a shared custom retry policy.
    ///
    /// # Panics
    ///
    /// Panics when another retry policy was already selected.
    pub fn custom_policy_arc(mut self, policy: Arc<dyn RetryPolicy>) -> Self {
        assert!(
            self.base_retry_policy.is_none(),
            "You have already selected another retry policy"
        );
        self.base_retry_policy = Some(policy);
        self
    }

    /// Uses an exponential back off.
    ///
    /// The wait of retry number `n` is `min(initial_interval * multiplier^n, max_interval)`.
    ///
    /// # Panics
    ///
    /// Panics when another back-off policy was already selected, when `initial_interval` is
    /// below one, when `multiplier` is not above one, or when `max_interval` is not above
    /// `initial_interval`.
    pub fn exponential_backoff(
        self,
        initial_interval: u64,
        multiplier: f64,
        max_interval: u64,
    ) -> Self {
        self.exponential_backoff_with_random(initial_interval, multiplier, max_interval, false)
    }

    /// Uses an exponential back off, optionally with jitter.
    ///
    /// # Panics
    ///
    /// See [`Self::exponential_backoff`].
    pub fn exponential_backoff_with_random(
        self,
        initial_interval: u64,
        multiplier: f64,
        max_interval: u64,
        with_random: bool,
    ) -> Self {
        assert!(
            self.back_off_policy.is_none(),
            "You have already selected backoff policy"
        );
        assert!(initial_interval >= 1, "Initial interval should be >= 1");
        assert!(multiplier > 1.0, "Multiplier should be > 1");
        assert!(
            max_interval > initial_interval,
            "Max interval should be > than initial interval"
        );

        let policy = if with_random {
            let mut policy = ExponentialRandomBackOffPolicy::new();
            policy.inner_mut().set_initial_interval(initial_interval);
            policy.inner_mut().set_multiplier(multiplier);
            policy.inner_mut().set_max_interval(max_interval);
            return self.set_back_off(Arc::new(policy));
        } else {
            let mut policy = ExponentialBackOffPolicy::new();
            policy.set_initial_interval(initial_interval);
            policy.set_multiplier(multiplier);
            policy.set_max_interval(max_interval);
            policy
        };

        self.set_back_off(Arc::new(policy))
    }

    /// Uses an exponential back off configured with durations.
    ///
    /// # Panics
    ///
    /// See [`Self::exponential_backoff`].
    pub fn exponential_backoff_from_duration(
        self,
        initial_interval: Duration,
        multiplier: f64,
        max_interval: Duration,
    ) -> Self {
        self.exponential_backoff(
            u64::try_from(initial_interval.as_millis()).unwrap_or(u64::MAX),
            multiplier,
            u64::try_from(max_interval.as_millis()).unwrap_or(u64::MAX),
        )
    }

    /// Waits for a fixed period before every retry.
    ///
    /// # Panics
    ///
    /// Panics when another back-off policy was already selected or when the interval is
    /// below one millisecond.
    pub fn fixed_backoff(mut self, interval: u64) -> Self {
        assert!(
            self.back_off_policy.is_none(),
            "You have already selected backoff policy"
        );
        assert!(interval >= 1, "Interval should be >= 1");
        let mut policy = FixedBackOffPolicy::new();
        policy.set_back_off_period(interval);
        self.back_off_policy = Some(Arc::new(policy));
        self
    }

    /// Waits for a fixed period configured with a duration.
    ///
    /// # Panics
    ///
    /// Panics when the interval rounds down to below one millisecond.
    pub fn fixed_backoff_from_duration(self, interval: Duration) -> Self {
        let millis = u64::try_from(interval.as_millis()).unwrap_or(u64::MAX);
        assert!(millis >= 1, "interval is less than 1 millisecond");
        self.fixed_backoff(millis)
    }

    /// Waits for a random period between two bounds before every retry.
    ///
    /// # Panics
    ///
    /// Panics when another back-off policy was already selected, when either bound is below
    /// one, or when the maximum is not above the minimum.
    pub fn uniform_random_backoff(mut self, min_interval: u64, max_interval: u64) -> Self {
        assert!(
            self.back_off_policy.is_none(),
            "You have already selected backoff policy"
        );
        assert!(min_interval >= 1, "Min interval should be >= 1");
        assert!(max_interval >= 1, "Max interval should be >= 1");
        assert!(
            max_interval > min_interval,
            "Max interval should be > than min interval"
        );

        let mut policy = UniformRandomBackOffPolicy::new();
        policy.set_min_back_off_period(min_interval);
        policy.set_max_back_off_period(max_interval);
        self.back_off_policy = Some(Arc::new(policy));
        self
    }

    /// Waits for a random period configured with durations.
    ///
    /// # Panics
    ///
    /// See [`Self::uniform_random_backoff`].
    pub fn uniform_random_backoff_from_duration(
        self,
        min_interval: Duration,
        max_interval: Duration,
    ) -> Self {
        self.uniform_random_backoff(
            u64::try_from(min_interval.as_millis()).unwrap_or(u64::MAX),
            u64::try_from(max_interval.as_millis()).unwrap_or(u64::MAX),
        )
    }

    /// Retries immediately without pausing.
    ///
    /// # Panics
    ///
    /// Panics when another back-off policy was already selected.
    pub fn no_backoff(mut self) -> Self {
        assert!(
            self.back_off_policy.is_none(),
            "You have already selected backoff policy"
        );
        self.back_off_policy = Some(Arc::new(NoBackOffPolicy::new()));
        self
    }

    /// Uses a custom back-off policy.
    ///
    /// # Panics
    ///
    /// Panics when another back-off policy was already selected.
    pub fn custom_backoff<T>(mut self, back_off_policy: T) -> Self
    where
        T: BackOffPolicy + 'static,
    {
        assert!(
            self.back_off_policy.is_none(),
            "You have already selected backoff policy"
        );
        self.back_off_policy = Some(Arc::new(back_off_policy));
        self
    }

    /// Uses a shared custom back-off policy.
    ///
    /// # Panics
    ///
    /// Panics when another back-off policy was already selected.
    pub fn custom_backoff_arc(mut self, back_off_policy: Arc<dyn BackOffPolicy>) -> Self {
        assert!(
            self.back_off_policy.is_none(),
            "You have already selected backoff policy"
        );
        self.back_off_policy = Some(back_off_policy);
        self
    }

    /// Adds an error that must be retried.
    ///
    /// `retry_on` and `not_retry_on` are mutually exclusive.
    ///
    /// # Panics
    ///
    /// Panics when [`Self::not_retry_on`] was used before.
    pub fn retry_on(mut self, error: RetryError) -> Self {
        self.classifier_builder_mut().retry_on(error);
        self
    }

    /// Adds errors that must be retried.
    ///
    /// # Panics
    ///
    /// Panics when [`Self::not_retry_on`] was used before.
    pub fn retry_on_all<I>(mut self, errors: I) -> Self
    where
        I: IntoIterator<Item = RetryError>,
    {
        for error in errors {
            self.classifier_builder_mut().retry_on(error);
        }
        self
    }

    /// Adds an error that must not be retried.
    ///
    /// # Panics
    ///
    /// Panics when [`Self::retry_on`] was used before.
    pub fn not_retry_on(mut self, error: RetryError) -> Self {
        self.classifier_builder_mut().no_retry_on(error);
        self
    }

    /// Adds errors that must not be retried.
    ///
    /// # Panics
    ///
    /// Panics when [`Self::retry_on`] was used before.
    pub fn not_retry_on_all<I>(mut self, errors: I) -> Self
    where
        I: IntoIterator<Item = RetryError>,
    {
        for error in errors {
            self.classifier_builder_mut().no_retry_on(error);
        }
        self
    }

    /// Inspects exception causes when classifying an error.
    ///
    /// Without it, an error that merely wraps a retryable one is not retried.
    pub fn traversing_causes(mut self) -> Self {
        self.classifier_builder_mut().traverse_causes = true;
        self
    }

    /// Retries every error whose concrete type is `T`.
    ///
    /// The rules created by the `*_error_type` and `*_error_if` methods classify by Rust type
    /// and fields instead of by the rendered message, which is what a service that owns its
    /// error types should use.
    ///
    /// Like the value-based rules, the two styles are exclusive: a list of errors to retry makes
    /// everything else non-retryable, and a list of errors not to retry makes everything else
    /// retryable. The rules also cannot be mixed with [`Self::retry_on`] or
    /// [`Self::retry_on_predicate`].
    ///
    /// # Panics
    ///
    /// Panics when a rule of the other style, a value-based rule or a predicate was already
    /// configured.
    pub fn retry_on_error_type<T>(mut self) -> Self
    where
        T: AnyError + 'static,
    {
        self.error_type_classifier_for(true).retry_on::<T>();
        self
    }

    /// Never retries an error whose concrete type is `T`.
    ///
    /// # Panics
    ///
    /// Panics when a rule of the other style, a value-based rule or a predicate was already
    /// configured.
    pub fn not_retry_on_error_type<T>(mut self) -> Self
    where
        T: AnyError + 'static,
    {
        self.error_type_classifier_for(false).no_retry_on::<T>();
        self
    }

    /// Retries errors of type `T` whose fields satisfy `predicate`.
    ///
    /// This is the field-level counterpart of [`Self::retry_on_error_type`]: for example,
    /// retry an HTTP error only when its status is 5xx.
    ///
    /// # Panics
    ///
    /// Panics when a rule of the other style, a value-based rule or a predicate was already
    /// configured.
    pub fn retry_on_error_if<T, F>(mut self, predicate: F) -> Self
    where
        T: AnyError + 'static,
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        self.error_type_classifier_for(true)
            .retry_on_if::<T, F>(predicate);
        self
    }

    /// Never retries errors of type `T` whose fields satisfy `predicate`.
    ///
    /// # Panics
    ///
    /// Panics when a rule of the other style, a value-based rule or a predicate was already
    /// configured.
    pub fn not_retry_on_error_if<T, F>(mut self, predicate: F) -> Self
    where
        T: AnyError + 'static,
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        self.error_type_classifier_for(false)
            .no_retry_on_if::<T, F>(predicate);
        self
    }

    /// Inspects the cause chain when classifying by type.
    ///
    /// Without it only the error itself, and the payload a [`RetryError::Any`] wraps, are
    /// inspected.
    pub fn traversing_error_causes(mut self) -> Self {
        self.error_type_traverse_causes = true;
        if let Some(classifier) = self.error_type_classifier.as_mut() {
            classifier.set_traverse_causes(true);
        }
        self
    }

    /// Decides with a predicate instead of a classifier.
    ///
    /// # Panics
    ///
    /// Panics when a classifier rule was already added, because the two cannot be mixed.
    pub fn retry_on_predicate(mut self, predicate: Arc<dyn Predicate<RetryError>>) -> Self {
        assert!(
            self.classifier_builder.is_none()
                && self.retry_on_predicate.is_none()
                && self.error_type_classifier.is_none(),
            "retry_on_predicate() cannot be mixed with the other classification rules"
        );
        self.retry_on_predicate = Some(predicate);
        self
    }

    /// Appends a listener.
    pub fn with_listener<T>(mut self, listener: T) -> Self
    where
        T: RetryListener + 'static,
    {
        self.listeners.push(Arc::new(listener));
        self
    }

    /// Appends listeners.
    pub fn with_listeners(mut self, listeners: Vec<Arc<dyn RetryListener>>) -> Self {
        self.listeners.extend(listeners);
        self
    }

    /// Sets whether a stateful retry rethrows the last error when it is exhausted.
    pub fn with_throw_last_exception_on_exhausted(mut self, value: bool) -> Self {
        self.throw_last_exception_on_exhausted = value;
        self
    }

    fn classifier_builder_mut(&mut self) -> &mut BinaryErrorClassifierBuilder {
        assert!(
            self.error_type_classifier.is_none() && self.retry_on_predicate.is_none(),
            "the value-based classification rules cannot be mixed with the type-based ones"
        );
        self.classifier_builder
            .get_or_insert_with(BinaryErrorClassifierBuilder::default)
    }

    fn error_type_classifier_for(&mut self, white_list: bool) -> &mut ErrorTypeClassifier {
        assert!(
            self.classifier_builder.is_none() && self.retry_on_predicate.is_none(),
            "the type-based classification rules cannot be mixed with the value-based ones"
        );
        assert!(
            self.error_type_is_white_list
                .map(|existing| existing == white_list)
                .unwrap_or(true),
            "Please use only retry_on_* or only not_retry_on_*"
        );
        self.error_type_is_white_list = Some(white_list);

        let traverse_causes = self.error_type_traverse_causes;
        let classifier = self
            .error_type_classifier
            .get_or_insert_with(|| ErrorTypeClassifier::new(!white_list));
        classifier.set_traverse_causes(traverse_causes);
        classifier
    }

    fn set_back_off(mut self, policy: Arc<dyn BackOffPolicy>) -> Self {
        self.back_off_policy = Some(policy);
        self
    }

    /// Builds the template.
    ///
    /// # Panics
    ///
    /// Panics when a classifier rule set is empty or mixes the two rule styles; both are
    /// configuration errors.
    pub fn build(mut self) -> RetryTemplate {
        let base_retry_policy = self
            .base_retry_policy
            .take()
            .unwrap_or_else(|| Arc::new(MaxAttemptsRetryPolicy::default()));

        let exception_retry_policy: Arc<dyn RetryPolicy> = if let Some(predicate) =
            self.retry_on_predicate.take()
        {
            Arc::new(PredicateRetryPolicy::new(predicate))
        } else if let Some(classifier) = self.error_type_classifier.take() {
            Arc::new(PredicateRetryPolicy::new(classifier.into_predicate()))
        } else {
            let classifier = if let Some(builder) = self.classifier_builder.take() {
                builder.build()
            } else {
                crate::classifier::binary_error_classifier::BinaryErrorClassifier::default_classifier()
            };
            Arc::new(BinaryErrorClassifierRetryPolicy::new(classifier))
        };

        let mut final_policy = CompositeRetryPolicy::new();
        final_policy.set_policies(vec![base_retry_policy, exception_retry_policy]);

        let mut retry_template = RetryTemplate::default();
        retry_template.set_retry_policy(final_policy);

        let back_off_policy = self
            .back_off_policy
            .take()
            .unwrap_or_else(|| Arc::new(NoBackOffPolicy::new()));
        retry_template.set_back_off_policy_arc(back_off_policy);

        if !self.listeners.is_empty() {
            retry_template.set_listeners(self.listeners);
        }

        retry_template
            .set_throw_last_exception_on_exhausted(self.throw_last_exception_on_exhausted);

        retry_template
    }
}

#[cfg(test)]
mod tests {
    use super::RetryTemplateBuilder;

    #[test]
    fn the_builder_defaults_to_three_attempts() {
        let template = RetryTemplateBuilder::new().build();

        assert_eq!(template.retry_policy().get_max_attempts(), 3);
    }

    #[test]
    fn a_timeout_policy_has_no_attempt_limit() {
        let template = RetryTemplateBuilder::new().with_timeout(500).build();

        assert_eq!(template.retry_policy().get_max_attempts(), 0);
    }

    #[test]
    #[should_panic(expected = "Number of attempts should be positive")]
    fn zero_attempts_is_rejected() {
        let _ = RetryTemplateBuilder::new().max_attempts(0);
    }

    #[test]
    #[should_panic(expected = "You have already selected another retry policy")]
    fn selecting_two_retry_policies_is_rejected() {
        let _ = RetryTemplateBuilder::new().max_attempts(2).infinite_retry();
    }

    #[test]
    #[should_panic(expected = "You have already selected backoff policy")]
    fn selecting_two_back_off_policies_is_rejected() {
        let _ = RetryTemplateBuilder::new().fixed_backoff(10).no_backoff();
    }

    #[test]
    #[should_panic(expected = "Max interval should be > than initial interval")]
    fn an_exponential_maximum_below_the_initial_interval_is_rejected() {
        let _ = RetryTemplateBuilder::new().exponential_backoff(100, 2.0, 50);
    }

    #[test]
    #[should_panic(expected = "Multiplier should be > 1")]
    fn an_exponential_multiplier_of_one_is_rejected() {
        let _ = RetryTemplateBuilder::new().exponential_backoff(100, 1.0, 500);
    }

    #[test]
    fn errors_and_back_off_options_are_recorded() {
        let template = RetryTemplateBuilder::new()
            .max_attempts(5)
            .fixed_backoff(20)
            .not_retry_on(crate::error::retry_error::RetryError::custom("fatal"))
            .build();

        assert_eq!(template.retry_policy().get_max_attempts(), 5);
    }

    #[test]
    fn a_duration_timeout_is_accepted() {
        let template = RetryTemplateBuilder::new()
            .with_timeout_from_duration(std::time::Duration::from_millis(250))
            .build();

        assert_eq!(template.retry_policy().get_max_attempts(), 0);
    }
}
