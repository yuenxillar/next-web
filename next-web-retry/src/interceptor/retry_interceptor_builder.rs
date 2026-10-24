//! Builds the stateless, stateful and circuit-breaker interceptors.

use std::sync::Arc;

use crate::{
    Predicate,
    backoff::{
        back_off_policy::BackOffPolicy, exponential_back_off_policy::ExponentialBackOffPolicy,
    },
    error::retry_error::RetryError,
    interceptor::{
        method_arguments_key_generator::MethodArgumentsKeyGenerator,
        method_invocation_recoverer::MethodInvocationRecoverer,
        new_method_arguments_identifier::NewMethodArgumentsIdentifier,
        retry_operations_interceptor::RetryOperationsInterceptor,
        stateful_retry_operations_interceptor::StatefulRetryOperationsInterceptor,
    },
    policy::simple_retry_policy::SimpleRetryPolicy,
    retry_policy::RetryPolicy,
    support::retry_template::RetryTemplate,
};

/// Entry point for building interceptors.
///
/// The three constructors mirror the three interceptors: stateless, stateful and
/// circuit-breaker. The return type of the intercepted call is the type parameter, so a call
/// that produces a `String` is intercepted with
/// `RetryInterceptorBuilder::stateless::<String>()`.
///
/// # Example
///
/// ```
/// use next_web_retry::interceptor::retry_interceptor_builder::RetryInterceptorBuilder;
///
/// let interceptor = RetryInterceptorBuilder::stateless::<u64>()
///     .max_attempts(3)
///     .back_off_options(10, 2.0, 100)
///     .build();
/// # let _ = interceptor;
/// ```
pub struct RetryInterceptorBuilder;

impl RetryInterceptorBuilder {
    /// Creates a builder for a stateless interceptor.
    pub fn stateless<T>() -> StatelessRetryInterceptorBuilder<T> {
        StatelessRetryInterceptorBuilder {
            common: CommonOptions::new(),
        }
    }

    /// Creates a builder for a stateful interceptor.
    pub fn stateful<T>() -> StatefulRetryInterceptorBuilder<T> {
        StatefulRetryInterceptorBuilder {
            common: CommonOptions::new(),
            key_generator: None,
            new_arguments_identifier: None,
            rollback_classifier: None,
        }
    }

    /// Creates a builder for a circuit-breaker interceptor.
    pub fn circuit_breaker<T>() -> CircuitBreakerInterceptorBuilder<T> {
        CircuitBreakerInterceptorBuilder {
            common: CommonOptions::new(),
            key_generator: None,
        }
    }
}

/// The options shared by the three interceptors.
struct CommonOptions<T> {
    retry_template: RetryTemplate,
    simple_retry_policy: SimpleRetryPolicy,
    retry_operations: Option<RetryTemplate>,
    recoverer: Option<Arc<dyn MethodInvocationRecoverer<T>>>,
    template_altered: bool,
    back_off_policy_set: bool,
    retry_policy_set: bool,
    back_off_options_set: bool,
    label: Option<String>,
}

impl<T> CommonOptions<T> {
    fn new() -> Self {
        let simple_retry_policy = SimpleRetryPolicy::default();
        let mut retry_template = RetryTemplate::default();
        retry_template.set_retry_policy(simple_retry_policy.clone());

        Self {
            retry_template,
            simple_retry_policy,
            retry_operations: None,
            recoverer: None,
            template_altered: false,
            back_off_policy_set: false,
            retry_policy_set: false,
            back_off_options_set: false,
            label: None,
        }
    }

    fn retry_operations(&mut self, retry_operations: RetryTemplate) {
        assert!(
            !self.template_altered,
            "Cannot set retry_operations when the default has been modified"
        );
        self.retry_operations = Some(retry_operations);
    }

    fn max_attempts(&mut self, max_attempts: u16) {
        assert!(
            self.retry_operations.is_none(),
            "cannot alter the retry policy when a custom retry_operations has been set"
        );
        assert!(
            !self.retry_policy_set,
            "cannot alter the retry policy when a custom retry_policy has been set"
        );
        self.simple_retry_policy.set_max_attempts(max_attempts);
        self.retry_template
            .set_retry_policy(self.simple_retry_policy.clone());
        self.template_altered = true;
    }

    fn back_off_options(&mut self, initial_interval: u64, multiplier: f64, max_interval: u64) {
        assert!(
            self.retry_operations.is_none(),
            "cannot set the back off policy when a custom retry_operations has been set"
        );
        assert!(
            !self.back_off_policy_set,
            "cannot set the back off options when a back off policy has been set"
        );

        let mut policy = ExponentialBackOffPolicy::new();
        policy.set_initial_interval(initial_interval);
        policy.set_multiplier(multiplier);
        policy.set_max_interval(max_interval);
        self.retry_template.set_back_off_policy(policy);
        self.back_off_options_set = true;
        self.template_altered = true;
    }

    fn retry_policy<P>(&mut self, policy: P)
    where
        P: RetryPolicy + 'static,
    {
        assert!(
            self.retry_operations.is_none(),
            "cannot set the retry policy when a custom retry_operations has been set"
        );
        assert!(
            !self.template_altered,
            "cannot set the retry policy if max attempts or back off policy or options changed"
        );
        self.retry_template.set_retry_policy(policy);
        self.retry_policy_set = true;
        self.template_altered = true;
    }

    fn back_off_policy<P>(&mut self, policy: P)
    where
        P: BackOffPolicy + 'static,
    {
        assert!(
            self.retry_operations.is_none(),
            "cannot set the back off policy when a custom retry_operations has been set"
        );
        assert!(
            !self.back_off_options_set,
            "cannot set the back off policy when the back off policy options have been set"
        );
        self.retry_template.set_back_off_policy(policy);
        self.back_off_policy_set = true;
        self.template_altered = true;
    }

    fn recoverer(&mut self, recoverer: Arc<dyn MethodInvocationRecoverer<T>>) {
        self.recoverer = Some(recoverer);
    }

    fn label(&mut self, label: impl Into<String>) {
        self.label = Some(label.into());
    }

    fn template(&self) -> RetryTemplate {
        match &self.retry_operations {
            Some(retry_operations) => retry_operations.clone(),
            None => self.retry_template.clone(),
        }
    }
}

/// Builds a [`RetryOperationsInterceptor`].
pub struct StatelessRetryInterceptorBuilder<T> {
    common: CommonOptions<T>,
}

impl<T> StatelessRetryInterceptorBuilder<T> {
    /// Uses a fully configured retry template.
    pub fn retry_operations(mut self, retry_operations: RetryTemplate) -> Self {
        self.common.retry_operations(retry_operations);
        self
    }

    /// Sets the maximum number of attempts.
    pub fn max_attempts(mut self, max_attempts: u16) -> Self {
        self.common.max_attempts(max_attempts);
        self
    }

    /// Configures an exponential back off.
    pub fn back_off_options(
        mut self,
        initial_interval: u64,
        multiplier: f64,
        max_interval: u64,
    ) -> Self {
        self.common
            .back_off_options(initial_interval, multiplier, max_interval);
        self
    }

    /// Uses a custom retry policy.
    pub fn retry_policy<P>(mut self, policy: P) -> Self
    where
        P: RetryPolicy + 'static,
    {
        self.common.retry_policy(policy);
        self
    }

    /// Uses a custom back-off policy.
    pub fn back_off_policy<P>(mut self, policy: P) -> Self
    where
        P: BackOffPolicy + 'static,
    {
        self.common.back_off_policy(policy);
        self
    }

    /// Installs a recoverer.
    pub fn recoverer(mut self, recoverer: Arc<dyn MethodInvocationRecoverer<T>>) -> Self {
        self.common.recoverer(recoverer);
        self
    }

    /// Sets the reporting label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.common.label(label);
        self
    }

    /// Builds the interceptor.
    pub fn build(self) -> RetryOperationsInterceptor<T> {
        let mut interceptor = RetryOperationsInterceptor::new(self.common.template());
        if let Some(recoverer) = self.common.recoverer {
            interceptor.set_recoverer(recoverer);
        }
        if let Some(label) = self.common.label {
            interceptor.set_label(label);
        }
        interceptor
    }
}

/// Builds a [`StatefulRetryOperationsInterceptor`].
pub struct StatefulRetryInterceptorBuilder<T> {
    common: CommonOptions<T>,
    key_generator: Option<Arc<dyn MethodArgumentsKeyGenerator>>,
    new_arguments_identifier: Option<Arc<dyn NewMethodArgumentsIdentifier>>,
    rollback_classifier: Option<Arc<dyn Predicate<RetryError>>>,
}

impl<T> StatefulRetryInterceptorBuilder<T> {
    /// Uses a fully configured retry template.
    pub fn retry_operations(mut self, retry_operations: RetryTemplate) -> Self {
        self.common.retry_operations(retry_operations);
        self
    }

    /// Sets the maximum number of attempts.
    pub fn max_attempts(mut self, max_attempts: u16) -> Self {
        self.common.max_attempts(max_attempts);
        self
    }

    /// Configures an exponential back off.
    pub fn back_off_options(
        mut self,
        initial_interval: u64,
        multiplier: f64,
        max_interval: u64,
    ) -> Self {
        self.common
            .back_off_options(initial_interval, multiplier, max_interval);
        self
    }

    /// Uses a custom retry policy.
    pub fn retry_policy<P>(mut self, policy: P) -> Self
    where
        P: RetryPolicy + 'static,
    {
        self.common.retry_policy(policy);
        self
    }

    /// Uses a custom back-off policy.
    pub fn back_off_policy<P>(mut self, policy: P) -> Self
    where
        P: BackOffPolicy + 'static,
    {
        self.common.back_off_policy(policy);
        self
    }

    /// Installs a recoverer.
    pub fn recoverer(mut self, recoverer: Arc<dyn MethodInvocationRecoverer<T>>) -> Self {
        self.common.recoverer(recoverer);
        self
    }

    /// Sets the reporting label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.common.label(label);
        self
    }

    /// Installs the key generator that identifies the item being retried.
    pub fn key_generator(mut self, key_generator: Arc<dyn MethodArgumentsKeyGenerator>) -> Self {
        self.key_generator = Some(key_generator);
        self
    }

    /// Installs the identifier that recognises never-seen arguments.
    pub fn new_arguments_identifier(
        mut self,
        identifier: Arc<dyn NewMethodArgumentsIdentifier>,
    ) -> Self {
        self.new_arguments_identifier = Some(identifier);
        self
    }

    /// Installs the rollback classifier.
    ///
    /// Every failure rolls back by default; a classifier narrows that down.
    pub fn rollback_for(mut self, classifier: Arc<dyn Predicate<RetryError>>) -> Self {
        self.rollback_classifier = Some(classifier);
        self
    }

    /// Builds the interceptor.
    pub fn build(self) -> StatefulRetryOperationsInterceptor<T> {
        let mut interceptor = StatefulRetryOperationsInterceptor::new(self.common.template());
        if let Some(recoverer) = self.common.recoverer {
            interceptor.set_recoverer(recoverer);
        }
        if let Some(key_generator) = self.key_generator {
            interceptor.set_key_generator(key_generator);
        }
        if let Some(identifier) = self.new_arguments_identifier {
            interceptor.set_new_arguments_identifier(identifier);
        }
        if let Some(classifier) = self.rollback_classifier {
            interceptor.set_rollback_classifier(classifier);
        }
        if let Some(label) = self.common.label {
            interceptor.set_label(label);
        }
        interceptor
    }
}

/// Builds a [`StatefulRetryOperationsInterceptor`] configured as a circuit breaker.
///
/// The only difference from [`StatefulRetryInterceptorBuilder`] is the rollback classifier: a
/// circuit breaker never rolls back, because a rollback would discard the circuit state that
/// is kept in the retry context.
pub struct CircuitBreakerInterceptorBuilder<T> {
    common: CommonOptions<T>,
    key_generator: Option<Arc<dyn MethodArgumentsKeyGenerator>>,
}

impl<T> CircuitBreakerInterceptorBuilder<T> {
    /// Uses a fully configured retry template.
    pub fn retry_operations(mut self, retry_operations: RetryTemplate) -> Self {
        self.common.retry_operations(retry_operations);
        self
    }

    /// Sets the maximum number of attempts.
    pub fn max_attempts(mut self, max_attempts: u16) -> Self {
        self.common.max_attempts(max_attempts);
        self
    }

    /// Configures an exponential back off.
    pub fn back_off_options(
        mut self,
        initial_interval: u64,
        multiplier: f64,
        max_interval: u64,
    ) -> Self {
        self.common
            .back_off_options(initial_interval, multiplier, max_interval);
        self
    }

    /// Uses a custom retry policy.
    pub fn retry_policy<P>(mut self, policy: P) -> Self
    where
        P: RetryPolicy + 'static,
    {
        self.common.retry_policy(policy);
        self
    }

    /// Uses a custom back-off policy.
    pub fn back_off_policy<P>(mut self, policy: P) -> Self
    where
        P: BackOffPolicy + 'static,
    {
        self.common.back_off_policy(policy);
        self
    }

    /// Installs a recoverer.
    pub fn recoverer(mut self, recoverer: Arc<dyn MethodInvocationRecoverer<T>>) -> Self {
        self.common.recoverer(recoverer);
        self
    }

    /// Sets the reporting label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.common.label(label);
        self
    }

    /// Installs the key generator that identifies the item being retried.
    pub fn key_generator(mut self, key_generator: Arc<dyn MethodArgumentsKeyGenerator>) -> Self {
        self.key_generator = Some(key_generator);
        self
    }

    /// Builds the interceptor.
    pub fn build(self) -> StatefulRetryOperationsInterceptor<T> {
        let mut interceptor = StatefulRetryOperationsInterceptor::new(self.common.template());
        if let Some(recoverer) = self.common.recoverer {
            interceptor.set_recoverer(recoverer);
        }
        if let Some(key_generator) = self.key_generator {
            interceptor.set_key_generator(key_generator);
        }
        if let Some(label) = self.common.label {
            interceptor.set_label(label);
        }
        interceptor.set_rollback_classifier(Arc::new(|_: &RetryError| false));
        interceptor
    }
}

#[cfg(test)]
mod tests {
    use super::RetryInterceptorBuilder;

    #[test]
    fn a_stateless_builder_applies_the_attempt_limit() {
        let interceptor = RetryInterceptorBuilder::stateless::<u64>()
            .max_attempts(4)
            .build();

        assert_eq!(
            interceptor
                .retry_operations()
                .retry_policy()
                .get_max_attempts(),
            4
        );
    }

    #[test]
    fn a_stateful_builder_returns_a_stateful_interceptor() {
        let interceptor = RetryInterceptorBuilder::stateful::<u64>()
            .max_attempts(2)
            .label("orders")
            .build();

        assert_eq!(
            interceptor
                .retry_operations()
                .retry_policy()
                .get_max_attempts(),
            2
        );
    }

    #[test]
    fn a_circuit_breaker_builder_returns_a_stateful_interceptor() {
        let interceptor = RetryInterceptorBuilder::circuit_breaker::<u64>()
            .max_attempts(2)
            .build();

        assert_eq!(
            interceptor
                .retry_operations()
                .retry_policy()
                .get_max_attempts(),
            2
        );
    }

    #[test]
    #[should_panic(expected = "Cannot set retry_operations when the default has been modified")]
    fn a_modified_template_rejects_custom_retry_operations() {
        let _ = RetryInterceptorBuilder::stateless::<u64>()
            .max_attempts(3)
            .retry_operations(crate::support::retry_template::RetryTemplate::default());
    }
}
