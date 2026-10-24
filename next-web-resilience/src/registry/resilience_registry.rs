//! The registry of named resilience instances.
//!
//! A pattern is only meaningful if every caller shares one instance: a circuit
//! breaker built per request would never see a failure rate, and a rate limiter
//! built per request would never reach its limit. The registry is where those
//! shared instances live.

use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};

use tracing::error;

use dashmap::DashMap;

use crate::{
    bulkhead::{Bulkhead, BulkheadConfig},
    circuitbreaker::{CircuitBreaker, CircuitBreakerConfig},
    ratelimiter::{RateLimiter, RateLimiterConfig},
    retry::{Retry, RetryConfig},
    timelimiter::{TimeLimiter, TimeLimiterConfig},
};

/// Holds one shared instance of each pattern, keyed by resource name.
///
/// Every `get` method returns the instance already registered under the name,
/// or creates it from the configuration the caller supplies. The configuration
/// is only built when the instance is actually missing, so a closure may be
/// used to defer the work — and, for a pattern whose configuration needs to
/// register listeners, to avoid doing it twice.
///
/// ```
/// use next_web_resilience::circuitbreaker::CircuitBreakerConfig;
/// use next_web_resilience::registry::ResilienceRegistry;
///
/// let registry = ResilienceRegistry::new();
///
/// // Created on first use, shared afterwards.
/// let breaker = registry.circuit_breaker("payments", || {
///     CircuitBreakerConfig::builder()
///         .sliding_window_size(50)
///         .minimum_number_of_calls(10)
///         .build()
/// });
/// let same = registry.circuit_breaker_or_default("payments");
///
/// assert!(std::sync::Arc::ptr_eq(&breaker, &same));
/// ```
#[derive(Debug, Default)]
pub struct ResilienceRegistry {
    circuit_breakers: DashMap<String, Arc<CircuitBreaker>>,
    rate_limiters: DashMap<String, Arc<RateLimiter>>,
    bulkheads: DashMap<String, Arc<Bulkhead>>,
    time_limiters: DashMap<String, Arc<TimeLimiter>>,
    retries: DashMap<String, Arc<Retry>>,
    /// Whether a name that is not registered is reported instead of being given
    /// a default instance in silence.
    strict_names: AtomicBool,
    /// The names already reported by [`Self::report_undeclared`], so that a
    /// repeated mistake is reported once rather than at request rate.
    reported_undeclared: DashMap<String, ()>,
}

impl ResilienceRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the process-wide registry.
    ///
    /// Shorthand for [`global_registry`].
    pub fn global() -> &'static Self {
        global_registry()
    }

    /// Sets whether a name that is not registered is refused rather than given a
    /// default instance.
    ///
    /// The `*_or_default` methods are what `name = "..."` on the attribute macros
    /// resolves through. Defaulting a name nobody configured means running with
    /// settings nobody chose — a limit of fifty a second where the file said five
    /// — and nothing says so. The `autoconfigure` feature turns this on when a
    /// configuration file declares instances, so a name the file does not declare
    /// is reported instead.
    ///
    /// Turn it back off if an application deliberately mixes configured instances
    /// with ad-hoc names.
    pub fn set_strict_names(&self, strict: bool) {
        self.strict_names.store(strict, Ordering::Release);
    }

    /// Returns whether an unregistered name is refused instead of defaulted.
    pub fn is_strict_names(&self) -> bool {
        self.strict_names.load(Ordering::Acquire)
    }

    /// Reports a name that had to be configured but is not.
    ///
    /// Reported **once per name**: this runs on the guarded call path, so
    /// repeating it would log at request rate.
    ///
    /// The call is **not** refused. A resilience library that took an endpoint
    /// down over a misspelled name would be causing the outage it exists to
    /// prevent, so the caller gets the defaults — which is wrong, but wrong in a
    /// way the log says out loud and the service survives. Refusing the call
    /// instead is not expressible here: a factory has to return an instance.
    fn report_undeclared(&self, pattern: &str, name: &str, registered: &[String]) {
        if self
            .reported_undeclared
            .insert(name.to_owned(), ())
            .is_some()
        {
            return;
        }

        error!(
            "no {pattern} named {name:?} is registered, so the defaults are in use. A \
             configuration file that declares instances does not have names defaulted: names in \
             the file are used exactly as written, so check the spelling. Registered: \
             {registered:?}"
        );
    }

    // -- circuit breakers -------------------------------------------------

    /// Returns the circuit breaker called `name`, creating it on first use from
    /// `config`.
    pub fn circuit_breaker<F>(&self, name: &str, config: F) -> Arc<CircuitBreaker>
    where
        F: FnOnce() -> CircuitBreakerConfig,
    {
        get_or_create(&self.circuit_breakers, name, || {
            CircuitBreaker::new(name, config())
        })
    }

    /// Returns the circuit breaker called `name`, creating it with the default
    /// configuration on first use.
    pub fn circuit_breaker_or_default(&self, name: &str) -> Arc<CircuitBreaker> {
        if self.is_strict_names() {
            return self.find_circuit_breaker(name).unwrap_or_else(|| {
                self.report_undeclared(
                    "circuit breaker",
                    name,
                    &self.circuit_breaker_names(),
                );
                self.circuit_breaker(name, CircuitBreakerConfig::default)
            });
        }

        self.circuit_breaker(name, CircuitBreakerConfig::default)
    }

    /// Registers a breaker that was built elsewhere, or returns the instance
    /// already registered under `name`.
    ///
    /// Used when the configuration comes from the application's properties rather
    /// than from a factory call, so that the instance the registry hands out is
    /// the very one that was configured.
    pub fn register_circuit_breaker(
        &self,
        name: &str,
        breaker: Arc<CircuitBreaker>,
    ) -> Arc<CircuitBreaker> {
        self.circuit_breakers
            .entry(name.to_owned())
            .or_insert(breaker)
            .clone()
    }

    /// Returns the circuit breaker called `name` if it is registered.
    pub fn find_circuit_breaker(&self, name: &str) -> Option<Arc<CircuitBreaker>> {
        find(&self.circuit_breakers, name)
    }

    /// Removes the circuit breaker called `name`.
    pub fn remove_circuit_breaker(&self, name: &str) -> Option<Arc<CircuitBreaker>> {
        remove(&self.circuit_breakers, name)
    }

    /// Returns the names of every registered circuit breaker.
    pub fn circuit_breaker_names(&self) -> Vec<String> {
        names(&self.circuit_breakers)
    }

    // -- rate limiters ----------------------------------------------------

    /// Returns the rate limiter called `name`, creating it on first use from
    /// `config`.
    pub fn rate_limiter<F>(&self, name: &str, config: F) -> Arc<RateLimiter>
    where
        F: FnOnce() -> RateLimiterConfig,
    {
        get_or_create(&self.rate_limiters, name, || RateLimiter::new(name, config()))
    }

    /// Returns the rate limiter called `name`, creating it with the default
    /// configuration on first use.
    pub fn rate_limiter_or_default(&self, name: &str) -> Arc<RateLimiter> {
        if self.is_strict_names() {
            return self.find_rate_limiter(name).unwrap_or_else(|| {
                self.report_undeclared("rate limiter", name, &self.rate_limiter_names());
                self.rate_limiter(name, RateLimiterConfig::default)
            });
        }

        self.rate_limiter(name, RateLimiterConfig::default)
    }

    /// Registers a limiter that was built elsewhere, or returns the instance
    /// already registered under `name`.
    ///
    /// Use this for a limiter whose budget lives in a shared store, which the
    /// [`Self::rate_limiter`] factory cannot build because it does not know the
    /// store. The caller is responsible for giving the limiter and the registry
    /// entry the same name; the two are not reconciled here.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use next_web_resilience::ratelimiter::{RateLimiter, RateLimiterConfig};
    /// use next_web_resilience::registry::ResilienceRegistry;
    ///
    /// let registry = ResilienceRegistry::new();
    /// let limiter = Arc::new(RateLimiter::new(
    ///     "login",
    ///     RateLimiterConfig::builder().limit_for_period(5).build(),
    /// ));
    ///
    /// let registered = registry.register_rate_limiter("login", limiter.clone());
    /// // The registry hands the same instance back from now on.
    /// assert!(Arc::ptr_eq(&registered, &limiter));
    /// assert!(Arc::ptr_eq(
    ///     &registry.rate_limiter_or_default("login"),
    ///     &limiter,
    /// ));
    /// ```
    pub fn register_rate_limiter(
        &self,
        name: &str,
        limiter: Arc<RateLimiter>,
    ) -> Arc<RateLimiter> {
        self.rate_limiters
            .entry(name.to_owned())
            .or_insert(limiter)
            .clone()
    }

    /// Returns the rate limiter called `name` if it is registered.
    pub fn find_rate_limiter(&self, name: &str) -> Option<Arc<RateLimiter>> {
        find(&self.rate_limiters, name)
    }

    /// Removes the rate limiter called `name`.
    pub fn remove_rate_limiter(&self, name: &str) -> Option<Arc<RateLimiter>> {
        remove(&self.rate_limiters, name)
    }

    /// Returns the names of every registered rate limiter.
    pub fn rate_limiter_names(&self) -> Vec<String> {
        names(&self.rate_limiters)
    }

    // -- bulkheads --------------------------------------------------------

    /// Returns the bulkhead called `name`, creating it on first use from
    /// `config`.
    pub fn bulkhead<F>(&self, name: &str, config: F) -> Arc<Bulkhead>
    where
        F: FnOnce() -> BulkheadConfig,
    {
        get_or_create(&self.bulkheads, name, || Bulkhead::new(name, config()))
    }

    /// Returns the bulkhead called `name`, creating it with the default
    /// configuration on first use.
    pub fn bulkhead_or_default(&self, name: &str) -> Arc<Bulkhead> {
        if self.is_strict_names() {
            return self.find_bulkhead(name).unwrap_or_else(|| {
                self.report_undeclared("bulkhead", name, &self.bulkhead_names());
                self.bulkhead(name, BulkheadConfig::default)
            });
        }

        self.bulkhead(name, BulkheadConfig::default)
    }

    /// Registers a bulkhead that was built elsewhere, or returns the instance
    /// already registered under `name`.
    pub fn register_bulkhead(&self, name: &str, bulkhead: Arc<Bulkhead>) -> Arc<Bulkhead> {
        self.bulkheads
            .entry(name.to_owned())
            .or_insert(bulkhead)
            .clone()
    }

    /// Returns the bulkhead called `name` if it is registered.
    pub fn find_bulkhead(&self, name: &str) -> Option<Arc<Bulkhead>> {
        find(&self.bulkheads, name)
    }

    /// Removes the bulkhead called `name`.
    pub fn remove_bulkhead(&self, name: &str) -> Option<Arc<Bulkhead>> {
        remove(&self.bulkheads, name)
    }

    /// Returns the names of every registered bulkhead.
    pub fn bulkhead_names(&self) -> Vec<String> {
        names(&self.bulkheads)
    }

    // -- time limiters ----------------------------------------------------

    /// Returns the time limiter called `name`, creating it on first use from
    /// `config`.
    pub fn time_limiter<F>(&self, name: &str, config: F) -> Arc<TimeLimiter>
    where
        F: FnOnce() -> TimeLimiterConfig,
    {
        get_or_create(&self.time_limiters, name, || {
            TimeLimiter::new(name, config())
        })
    }

    /// Returns the time limiter called `name`, creating it with the default
    /// configuration on first use.
    pub fn time_limiter_or_default(&self, name: &str) -> Arc<TimeLimiter> {
        if self.is_strict_names() {
            return self.find_time_limiter(name).unwrap_or_else(|| {
                self.report_undeclared("time limiter", name, &self.time_limiter_names());
                self.time_limiter(name, TimeLimiterConfig::default)
            });
        }

        self.time_limiter(name, TimeLimiterConfig::default)
    }

    /// Registers a time limiter that was built elsewhere, or returns the instance
    /// already registered under `name`.
    pub fn register_time_limiter(
        &self,
        name: &str,
        limiter: Arc<TimeLimiter>,
    ) -> Arc<TimeLimiter> {
        self.time_limiters
            .entry(name.to_owned())
            .or_insert(limiter)
            .clone()
    }

    /// Returns the time limiter called `name` if it is registered.
    pub fn find_time_limiter(&self, name: &str) -> Option<Arc<TimeLimiter>> {
        find(&self.time_limiters, name)
    }

    /// Removes the time limiter called `name`.
    pub fn remove_time_limiter(&self, name: &str) -> Option<Arc<TimeLimiter>> {
        remove(&self.time_limiters, name)
    }

    /// Returns the names of every registered time limiter.
    pub fn time_limiter_names(&self) -> Vec<String> {
        names(&self.time_limiters)
    }

    // -- retries ----------------------------------------------------------

    /// Returns the retry called `name`, creating it on first use from
    /// `config`.
    pub fn retry<F>(&self, name: &str, config: F) -> Arc<Retry>
    where
        F: FnOnce() -> RetryConfig,
    {
        get_or_create(&self.retries, name, || Retry::new(name, config()))
    }

    /// Returns the retry called `name`, creating it with the default
    /// configuration on first use.
    pub fn retry_or_default(&self, name: &str) -> Arc<Retry> {
        if self.is_strict_names() {
            return self.find_retry(name).unwrap_or_else(|| {
                self.report_undeclared("retry", name, &self.retry_names());
                self.retry(name, RetryConfig::default)
            });
        }

        self.retry(name, RetryConfig::default)
    }

    /// Registers a retry that was built elsewhere, or returns the instance
    /// already registered under `name`.
    pub fn register_retry(&self, name: &str, retry: Arc<Retry>) -> Arc<Retry> {
        self.retries
            .entry(name.to_owned())
            .or_insert(retry)
            .clone()
    }

    /// Returns the retry called `name` if it is registered.
    pub fn find_retry(&self, name: &str) -> Option<Arc<Retry>> {
        find(&self.retries, name)
    }

    /// Removes the retry called `name`.
    pub fn remove_retry(&self, name: &str) -> Option<Arc<Retry>> {
        remove(&self.retries, name)
    }

    /// Returns the names of every registered retry.
    pub fn retry_names(&self) -> Vec<String> {
        names(&self.retries)
    }

    // -- whole registry ---------------------------------------------------

    /// Removes every registered instance.
    ///
    /// Callers that still hold an [`Arc`] to a removed instance keep using it;
    /// the next lookup of that name creates a fresh one.
    pub fn clear(&self) {
        self.circuit_breakers.clear();
        self.rate_limiters.clear();
        self.bulkheads.clear();
        self.time_limiters.clear();
        self.retries.clear();
    }

    /// Returns the total number of registered instances.
    pub fn len(&self) -> usize {
        self.circuit_breakers.len()
            + self.rate_limiters.len()
            + self.bulkheads.len()
            + self.time_limiters.len()
            + self.retries.len()
    }

    /// Returns whether the registry holds no instance at all.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Returns the process-wide registry that the `name = "..."` form of the
/// attribute macros resolves its instances from.
///
/// The registry is created on the first call and lives for the rest of the
/// process, so every part of an application that names the same resource gets
/// the same instance — which is the point: a rate limiter built per call site
/// would never reach its own limit.
///
/// Because it is process-wide, a test that uses the named form cannot run beside
/// another test that registers the same name. Prefer the expression form of the
/// macros, or an explicit [`ResilienceRegistry`], wherever that matters.
///
/// An instance named here is created with the default configuration the first
/// time it is asked for. Register it before the first call if you want to choose
/// its settings:
///
/// ```
/// use next_web_resilience::circuitbreaker::CircuitBreakerConfig;
/// use next_web_resilience::registry::global_registry;
///
/// let registry = global_registry();
/// registry.circuit_breaker("payments", || {
///     CircuitBreakerConfig::builder()
///         .sliding_window_size(20)
///         .minimum_number_of_calls(10)
///         .build()
/// });
///
/// assert_eq!(
///     registry
///         .find_circuit_breaker("payments")
///         .expect("just registered")
///         .config()
///         .sliding_window_size(),
///     20
/// );
/// ```
pub fn global_registry() -> &'static ResilienceRegistry {
    static REGISTRY: OnceLock<ResilienceRegistry> = OnceLock::new();
    REGISTRY.get_or_init(ResilienceRegistry::new)
}

/// Returns the registered instance, or inserts the one `create` builds.
///
/// Taking the shard lock through the entry API keeps two concurrent first
/// callers from ending up with two different instances — and therefore two
/// different sets of counters — for the same name.
fn get_or_create<V, F>(map: &DashMap<String, Arc<V>>, name: &str, create: F) -> Arc<V>
where
    F: FnOnce() -> V,
{
    // The common case is a hit, which needs no allocation and no write lock.
    if let Some(existing) = map.get(name) {
        return Arc::clone(&existing);
    }

    let entry = map
        .entry(name.to_owned())
        .or_insert_with(|| Arc::new(create()));
    Arc::clone(&entry)
}

/// Returns a registered instance without creating one.
fn find<V>(map: &DashMap<String, Arc<V>>, name: &str) -> Option<Arc<V>> {
    map.get(name).map(|entry| Arc::clone(&entry))
}

/// Removes a registered instance.
fn remove<V>(map: &DashMap<String, Arc<V>>, name: &str) -> Option<Arc<V>> {
    map.remove(name).map(|(_, value)| value)
}

/// Returns the registered names.
fn names<V>(map: &DashMap<String, Arc<V>>) -> Vec<String> {
    let mut names: Vec<String> = map.iter().map(|entry| entry.key().clone()).collect();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::ResilienceRegistry;
    use crate::{
        bulkhead::BulkheadConfig,
        circuitbreaker::CircuitBreakerConfig,
        ratelimiter::RateLimiterConfig,
        retry::RetryConfig,
        timelimiter::TimeLimiterConfig,
    };

    #[test]
    fn a_breaker_is_created_once_and_shared_afterwards() {
        let registry = ResilienceRegistry::new();

        let first = registry.circuit_breaker("payments", || {
            CircuitBreakerConfig::builder().sliding_window_size(7).build()
        });
        let second = registry.circuit_breaker("payments", || {
            CircuitBreakerConfig::builder().sliding_window_size(99).build()
        });

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(second.config().sliding_window_size(), 7);
    }

    #[test]
    fn the_configuration_closure_is_not_called_when_the_instance_exists() {
        let registry = ResilienceRegistry::new();
        let _ = registry.circuit_breaker_or_default("payments");

        let built = registry.circuit_breaker("payments", || {
            panic!("the configuration must not be built again")
        });

        assert_eq!(built.name(), "payments");
    }

    #[test]
    fn every_pattern_has_its_own_namespace() {
        let registry = ResilienceRegistry::new();

        // The same name can be used by each pattern without collisions.
        let _ = registry.circuit_breaker_or_default("shared");
        let _ = registry.rate_limiter_or_default("shared");
        let _ = registry.bulkhead_or_default("shared");
        let _ = registry.time_limiter_or_default("shared");
        let _ = registry.retry_or_default("shared");

        assert_eq!(registry.len(), 5);
        assert_eq!(registry.circuit_breaker_names(), vec!["shared".to_owned()]);
        assert_eq!(registry.rate_limiter_names(), vec!["shared".to_owned()]);
        assert_eq!(registry.bulkhead_names(), vec!["shared".to_owned()]);
        assert_eq!(registry.time_limiter_names(), vec!["shared".to_owned()]);
        assert_eq!(registry.retry_names(), vec!["shared".to_owned()]);
    }

    #[test]
    fn instances_can_be_found_and_removed() {
        let registry = ResilienceRegistry::new();
        let created = registry.circuit_breaker_or_default("payments");

        let found = registry
            .find_circuit_breaker("payments")
            .expect("the breaker is registered");
        assert!(Arc::ptr_eq(&created, &found));

        let removed = registry
            .remove_circuit_breaker("payments")
            .expect("the breaker is registered");
        assert!(Arc::ptr_eq(&created, &removed));
        assert!(registry.find_circuit_breaker("payments").is_none());
        assert!(registry.is_empty());
    }

    #[test]
    fn a_removed_breaker_is_created_again_on_the_next_lookup() {
        let registry = ResilienceRegistry::new();
        let first = registry.circuit_breaker_or_default("payments");
        let _ = registry.remove_circuit_breaker("payments");

        let second = registry.circuit_breaker_or_default("payments");

        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn an_empty_registry_reports_nothing() {
        let registry = ResilienceRegistry::new();

        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.circuit_breaker_names().is_empty());
        assert!(registry.find_retry("missing").is_none());
    }

    #[test]
    fn clearing_empties_the_registry() {
        let registry = ResilienceRegistry::new();
        let _ = registry.retry("inventory", RetryConfig::default);
        let _ = registry.bulkhead("inventory", BulkheadConfig::default);
        let _ = registry.rate_limiter("public-api", RateLimiterConfig::default);
        let _ = registry.time_limiter("search", TimeLimiterConfig::default);

        registry.clear();

        assert!(registry.is_empty());
    }

    #[test]
    fn names_are_sorted_so_a_report_is_stable() {
        let registry = ResilienceRegistry::new();
        let _ = registry.circuit_breaker_or_default("zulu");
        let _ = registry.circuit_breaker_or_default("alpha");
        let _ = registry.circuit_breaker_or_default("mike");

        assert_eq!(
            registry.circuit_breaker_names(),
            vec!["alpha".to_owned(), "mike".to_owned(), "zulu".to_owned()]
        );
    }
}
