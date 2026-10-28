//! Guarding methods with the attribute macros, on the instances
//! `resources/application.yaml` declares.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p next-web-examples --bin resilience_configured
//! ```
//!
//! then, in another shell:
//!
//! ```text
//! curl http://127.0.0.1:11000/app/resilience        # what the file declared
//! curl http://127.0.0.1:11000/app/resilience/demo   # the macros, on those instances
//! ```
//!
//! This is the merge of the two resilience examples the repository used to ship
//! separately: the one that declared its instances in `application.yaml`, and the
//! one that put the attribute macros through seven sections. They belong together
//! because the macros' named form is exactly what a configured instance is for.
//!
//! A plain binary cannot read the configuration file: the properties are bound
//! while an application starts (`WebAutoConfiguration` does it), so the second
//! example had to build its instances in code and the named form it showed
//! resolved a name nothing had registered — which quietly produced a default
//! instance. Here the sections run behind a route of the application that did read
//! the file, so by the time they run every `name = "..."` resolves a configured
//! instance.
//!
//! What the routes show:
//!
//! - `/resilience` reports the instances the configuration produced, straight from
//!   the process-wide registry;
//! - `/resilience/demo` runs the seven demonstrations and returns their report as
//!   the response body, preserving each section's explanation and numbers.
//!
//! What happens while the application starts:
//!
//! 1. `resources/application.yaml` is read and flattened;
//! 2. the properties under `next.ratelimiter`, `next.circuitbreaker`,
//!    `next.bulkhead`, `next.timelimiter` and `next.retry` are bound into the
//!    structures this crate ships;
//! 3. each instance is built from its pattern's builder, so a setting outside its
//!    range fails the start rather than the first call;
//! 4. every instance is registered in the process-wide registry under the name the
//!    file gave it, and in the application context behind the same `Arc`;
//! 5. from then on `name = "backendA"` on `#[rate_limiter]` resolves the instance
//!    the file described, and a name the file does not declare is reported rather
//!    than quietly given a default.
//!
//! That last point is the whole reason for the merge, and section 7 below is where
//! it is made explicit.

use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::Duration,
};

use next_web::{Application, NextWebApplication, macros::bind::get_mapping};
use next_web_resilience::{
    AbortedError, BulkheadFullError, CallNotPermittedError, RateLimitExceededError, TimeoutError,
    bulkhead,
    bulkhead::{Bulkhead, BulkheadConfig},
    circuit_breaker,
    circuitbreaker::{CircuitBreaker, CircuitBreakerConfig},
    rate_limiter,
    ratelimiter::{RateLimiter, RateLimiterConfig},
    registry::global_registry,
    retry,
    retry::{Retry, RetryConfig},
    time_limiter,
    timelimiter::{TimeLimiter, TimeLimiterConfig},
};

/// The session handed back when a login is refused and degraded to a guest.
const GUEST_SESSION: u32 = 0;

/// The application of this example.
#[derive(Default)]
struct ConfiguredApplication;

impl Application for ConfiguredApplication {}

/// Reports what the configuration file declared.
///
/// Reading the registry here is the point: the instances exist because the
/// application started, not because this handler built them. A name that is
/// missing from the registry at this point is a name the file did not declare.
#[get_mapping(path = "/resilience")]
async fn resilience() -> String {
    let registry = global_registry();
    let mut report = String::from("Instances registered from resources/application.yaml:\n");

    for name in registry.rate_limiter_names() {
        let limiter = registry
            .find_rate_limiter(&name)
            .expect("a name from the registry resolves");
        let config = limiter.config();

        report.push_str(&format!(
            "\n  rate limiter {name}\n    limit for period        : {}\n    limit refresh period    : {:?}\n    timeout duration        : {:?}\n",
            config.limit_for_period(),
            config.limit_refresh_period(),
            config.timeout_duration(),
        ));
    }

    for name in registry.circuit_breaker_names() {
        let breaker = registry
            .find_circuit_breaker(&name)
            .expect("a name from the registry resolves");
        let config = breaker.config();

        report.push_str(&format!(
            "\n  circuit breaker {name}\n    sliding window size     : {}\n    failure rate threshold  : {}\n    wait in open state      : {:?}\n    half-open permits       : {}\n",
            config.sliding_window_size(),
            config.failure_rate_threshold(),
            config.wait_duration_in_open_state(),
            config.permitted_number_of_calls_in_half_open_state(),
        ));
    }

    for name in registry.bulkhead_names() {
        let bulkhead = registry
            .find_bulkhead(&name)
            .expect("a name from the registry resolves");
        let config = bulkhead.config();

        report.push_str(&format!(
            "\n  bulkhead {name}\n    max concurrent calls    : {}\n    max wait duration       : {:?}\n",
            config.max_concurrent_calls(),
            config.max_wait_duration(),
        ));
    }

    for name in registry.time_limiter_names() {
        let limiter = registry
            .find_time_limiter(&name)
            .expect("a name from the registry resolves");
        let config = limiter.config();

        report.push_str(&format!(
            "\n  time limiter {name}\n    timeout duration        : {:?}\n    cancel running future   : {}\n",
            config.timeout_duration(),
            config.cancel_running_future(),
        ));
    }

    for name in registry.retry_names() {
        let retry = registry
            .find_retry(&name)
            .expect("a name from the registry resolves");
        let config = retry.config();

        report.push_str(&format!(
            "\n  retry {name}\n    max attempts            : {}\n    wait duration           : {:?}\n",
            config.max_attempts(),
            config.wait_duration(),
        ));
    }

    report.push_str(&format!(
        "\nUndeclared names are reported in the log, and the call is then served on the \
         defaults: {}",
        if registry.is_strict_names() {
            "yes, this application declared instances"
        } else {
            "no, this application declared none"
        }
    ));

    report
}

/// Runs the seven demonstrations and returns their report.
///
/// The report is the response body rather than stdout because this is an
/// application: the demonstrations need the instances the file declared, and
/// those only exist once the application has started. Nothing here reaches the
/// network or a database — every guard is around a local body — so the route
/// answers in about half a second, most of it section 5 waiting out its budget.
#[get_mapping(path = "/resilience/demo")]
async fn demo() -> String {
    let mut report = String::from(
        "Resilience attribute macros — every guard below is an attribute on a method.\n\
         Sections 1 to 6 guard instances built here; section 7 guards the instance \
         resources/application.yaml declared.\n",
    );

    happy_path(&mut report).await;
    retry_recovers(&mut report).await;
    degraded(&mut report).await;
    quota(&mut report).await;
    slow_call(&mut report).await;
    concurrency(&mut report).await;
    by_name(&mut report).await;

    report.push_str(
        "\nDone: five patterns, a stack of two, two fallbacks and the registry form.\n",
    );

    report
}

/// Appends a section heading.
fn section(report: &mut String, step: u32, title: &str) {
    report.push_str(&format!("\n── {step}. {title}\n"));
}

/// Renders an outcome for the section output.
fn describe<T: fmt::Debug>(outcome: &Result<T, GatewayError>) -> String {
    match outcome {
        Ok(value) => format!("Ok({value:?})"),
        Err(error) => format!("Err({error})"),
    }
}

/// 1. The simplest case: the breaker lets the call through and records it.
async fn happy_path(report: &mut String) {
    section(report, 1, "A guarded call on the happy path");

    let payments = Payments::new();
    // Pretend two attempts have already happened, so the next one succeeds.
    payments.calls.store(2, Ordering::SeqCst);

    let outcome = payments.charge().await;
    report.push_str(&format!("   charge            -> {}\n", describe(&outcome)));

    let snapshot = payments.breaker.snapshot();
    report.push_str(&format!(
        "   breaker           : buffered={}, succeeded={}, failed={}, state={}\n",
        snapshot.number_of_buffered_calls,
        snapshot.number_of_successful_calls,
        snapshot.number_of_failed_calls,
        snapshot.state
    ));
}

/// 2. The retry recovers a flaky call, and the breaker sees every attempt.
async fn retry_recovers(report: &mut String) {
    section(report, 2, "A retry recovers a flaky call, under a breaker");

    let payments = Payments::new();

    let outcome = payments.charge().await;
    report.push_str(&format!("   charge            -> {}\n", describe(&outcome)));
    report.push_str(&format!(
        "   attempts made     : {}\n",
        payments.calls.load(Ordering::SeqCst)
    ));

    let retry = payments.retry.snapshot();
    report.push_str(&format!(
        "   retry             : retries={}, succeeded-with-retry={}\n",
        retry.number_of_retries, retry.number_of_successful_calls_with_retry
    ));

    let breaker = payments.breaker.snapshot();
    report.push_str(&format!(
        "   breaker           : buffered={}, because `#[retry]` is written below \
         `#[circuit_breaker]` and therefore wraps it\n",
        breaker.number_of_buffered_calls
    ));
    report.push_str(
        "   note              : writing them the other way round makes the breaker wrap the \
         whole retry loop and count one call\n",
    );
}

/// 3. Once the breaker is open, the fallback answers instead of an error.
async fn degraded(report: &mut String) {
    section(report, 3, "A fallback degrades instead of failing");

    let payments = Payments::new();
    payments.fail(true);

    // Four failing calls are enough to open a breaker with this configuration;
    // the fifth never reaches the gateway.
    for call in 1..=5 {
        let outcome = payments.charge_or_defer().await;
        report.push_str(&format!(
            "   charge_or_defer {call} -> {}\n",
            describe(&outcome)
        ));
    }

    report.push_str(&format!(
        "   breaker state     : {}\n",
        payments.breaker.state()
    ));
    report.push_str(&format!(
        "   gateway calls     : {} — an open breaker never reaches it\n",
        payments.calls.load(Ordering::SeqCst)
    ));
}

/// 4. The limiter refuses past the quota, and a fallback covers for it.
async fn quota(report: &mut String) {
    section(report, 4, "A rate limiter refuses past its quota");

    let payments = Payments::new();

    for call in 1..=4 {
        let outcome = payments.login().await;
        report.push_str(&format!(
            "   login {call}          -> {}\n",
            describe(&outcome)
        ));
    }

    report.push_str("   ── the same spent quota, but the caller asked for the degraded form\n");
    let outcome = payments.login_or_guest().await;
    report.push_str(&format!(
        "   login_or_guest    -> {} (session {GUEST_SESSION} is the guest)\n",
        describe(&outcome)
    ));
    report.push_str(&format!(
        "   limiter           : acquired={}, rejected={}\n",
        payments.limiter.snapshot().number_of_acquired_permissions,
        payments.limiter.snapshot().number_of_rejected_permissions
    ));
}

/// 5. The time limiter abandons a call that outlives its budget.
async fn slow_call(report: &mut String) {
    section(report, 5, "A time limiter abandons a slow call");

    let payments = Payments::new();
    let outcome = payments.search().await;

    report.push_str(&format!("   search            -> {}\n", describe(&outcome)));
    report.push_str(&format!(
        "   time limiter      : timeouts={}, and {} guarded bodies reached their own end — \
         the future was dropped\n",
        payments.time_limiter.snapshot().number_of_timeouts,
        payments.calls.load(Ordering::SeqCst)
    ));
}

/// 6. The bulkhead caps how many calls run at once.
async fn concurrency(report: &mut String) {
    section(report, 6, "A bulkhead caps concurrency");

    let payments = Arc::new(Payments::new());

    let mut tasks = Vec::new();
    for _ in 0..6 {
        let payments = Arc::clone(&payments);
        tasks.push(tokio::spawn(async move { payments.report().await.is_ok() }));
    }

    let mut completed = 0;
    for task in tasks {
        if task.await.unwrap_or(false) {
            completed += 1;
        }
    }

    report.push_str(&format!("   {completed} of 6 reports completed\n"));
    report.push_str(&format!(
        "   peak concurrent   : {} (limit 2)\n",
        payments.peak.load(Ordering::SeqCst)
    ));
    report.push_str(&format!(
        "   bulkhead          : available={}, rejected={}\n",
        payments.bulkhead.available_concurrent_calls(),
        payments.bulkhead.snapshot().number_of_rejected_calls
    ));
}

/// The named form resolves a **configured** instance, which is the point of
/// merging the two examples.
///
/// The name below is one `resources/application.yaml` declares, and the limits
/// reported at the end are read back out of that instance, so the section states
/// where they came from rather than asserting it. The same method written against
/// the defaults — `example_reports_login`, the name the example that only knew
/// about macros used — would allow fifty calls per half second instead of five per
/// second, and nothing would have said so.
async fn by_name(report: &mut String) {
    section(
        report,
        7,
        "The same guards, resolved by name — from the configuration file",
    );

    struct Reports;

    impl Reports {
        /// No field to point at: the instance comes from the process registry,
        /// under the name `application.yaml` declared.
        #[rate_limiter(name = "backendA")]
        async fn login(&self) -> Result<u32, GatewayError> {
            Ok(1)
        }
    }

    let reports = Reports;
    let mut granted = 0;
    for _ in 0..6 {
        if reports.login().await.is_ok() {
            granted += 1;
        }
    }

    let registered = global_registry()
        .find_rate_limiter("backendA")
        .expect("the configuration file declared backendA");

    // The limits are read from the instance the macro resolved, not from a
    // constant: if the file changed and the instance did not, this section would
    // say so instead of agreeing with the file.
    report.push_str(&format!(
        "   {granted} of 6 calls fit backendA's quota of {} per {:?}, waiting {:?}\n",
        registered.config().limit_for_period(),
        registered.config().limit_refresh_period(),
        registered.config().timeout_duration()
    ));
    report.push_str(&format!(
        "   limiter counters  : acquired={}, rejected={}\n",
        registered.snapshot().number_of_acquired_permissions,
        registered.snapshot().number_of_rejected_permissions
    ));
    report.push_str(
        "   note              : these limits came from resources/application.yaml, not from the \
         defaults. The default quota is 50 per 500ms and would have granted every call, and the \
         name is registered because the application read the file at start-up — a plain binary \
         cannot, which is why this section lives behind a route. Calling this route again inside \
         the same one second period finds the budget already spent and gates fewer calls until \
         the period refreshes.\n",
    );
    report.push_str(
        "   note              : the expression form used everywhere else keeps the instance \
         visible to the code that builds it, which is what a test needs\n",
    );
}

/// A receipt for a charge that went through.
#[derive(Debug, PartialEq, Eq)]
struct Receipt {
    /// Identifier the gateway returned.
    id: u32,
    /// Whether this receipt stands in for a charge that has not happened yet.
    deferred: bool,
}

impl Receipt {
    /// A real receipt.
    fn charged(id: u32) -> Self {
        Self {
            id,
            deferred: false,
        }
    }

    /// The receipt handed back when the breaker is open: the charge is queued.
    fn deferred() -> Self {
        Self {
            id: 0,
            deferred: true,
        }
    }
}

/// Everything the guarded methods can fail with.
///
/// The `From` implementations are what let a macro convert a rejection into this
/// error when no `fallback` is given.
#[derive(Debug)]
enum GatewayError {
    /// The breaker refused the call.
    BreakerOpen,
    /// The limiter refused the call.
    Throttled,
    /// The bulkhead was full.
    Overloaded,
    /// The call took too long.
    TooSlow,
    /// The call ended without a result.
    Aborted,
    /// The gateway itself failed.
    Failed(String),
}

impl fmt::Display for GatewayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BreakerOpen => f.write_str("the circuit breaker is open"),
            Self::Throttled => f.write_str("the rate limiter refused the call"),
            Self::Overloaded => f.write_str("the bulkhead is full"),
            Self::TooSlow => f.write_str("the call took too long"),
            Self::Aborted => f.write_str("the call ended without a result"),
            Self::Failed(reason) => write!(f, "the gateway failed: {reason}"),
        }
    }
}

impl std::error::Error for GatewayError {}

impl From<CallNotPermittedError> for GatewayError {
    fn from(_: CallNotPermittedError) -> Self {
        Self::BreakerOpen
    }
}

impl From<RateLimitExceededError> for GatewayError {
    fn from(_: RateLimitExceededError) -> Self {
        Self::Throttled
    }
}

impl From<BulkheadFullError> for GatewayError {
    fn from(_: BulkheadFullError) -> Self {
        Self::Overloaded
    }
}

impl From<TimeoutError> for GatewayError {
    fn from(_: TimeoutError) -> Self {
        Self::TooSlow
    }
}

impl From<AbortedError> for GatewayError {
    fn from(_: AbortedError) -> Self {
        Self::Aborted
    }
}

/// A payment gateway, guarded by every pattern the crate provides.
///
/// Sections 1 to 6 guard these instances, built in code so the numbers they print
/// are reproducible on every call. The one section that has to resolve a
/// configured instance instead is section 7, which is why it carries none of
/// these fields.
struct Payments {
    /// Refuses calls once the failure rate crosses the threshold.
    breaker: Arc<CircuitBreaker>,
    /// Three logins per period.
    limiter: Arc<RateLimiter>,
    /// Two concurrent reports.
    bulkhead: Arc<Bulkhead>,
    /// Fifty milliseconds per search.
    time_limiter: Arc<TimeLimiter>,
    /// Three attempts per charge.
    retry: Arc<Retry>,
    /// How many times a guarded body ran.
    calls: Arc<AtomicU32>,
    /// How many guarded bodies are running right now.
    in_flight: Arc<AtomicU32>,
    /// The most that ever ran at once.
    peak: Arc<AtomicU32>,
    /// Whether the gateway should fail.
    failing: Arc<AtomicBool>,
}

impl Payments {
    /// Builds the gateway with configuration suited to a quick demonstration.
    fn new() -> Self {
        Self {
            breaker: Arc::new(CircuitBreaker::new(
                "payments",
                CircuitBreakerConfig::builder()
                    .sliding_window_size(4)
                    .minimum_number_of_calls(4)
                    .failure_rate_threshold(50.0)
                    .wait_duration_in_open_state(Duration::from_secs(30))
                    .build(),
            )),
            limiter: Arc::new(RateLimiter::new(
                "login",
                RateLimiterConfig::builder()
                    .limit_for_period(3)
                    .limit_refresh_period(Duration::from_secs(60))
                    .timeout_duration(Duration::ZERO)
                    .build(),
            )),
            bulkhead: Arc::new(Bulkhead::new(
                "reports",
                BulkheadConfig::builder()
                    .max_concurrent_calls(2)
                    .max_wait_duration(Duration::from_secs(2))
                    .build(),
            )),
            time_limiter: Arc::new(TimeLimiter::new(
                "search",
                TimeLimiterConfig::builder()
                    .timeout_duration(Duration::from_millis(50))
                    .build(),
            )),
            retry: Arc::new(Retry::new(
                "inventory",
                RetryConfig::builder()
                    .max_attempts(3)
                    .wait_duration(Duration::from_millis(5))
                    .build(),
            )),
            calls: Arc::new(AtomicU32::new(0)),
            in_flight: Arc::new(AtomicU32::new(0)),
            peak: Arc::new(AtomicU32::new(0)),
            failing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Makes the gateway fail, or stop failing.
    fn fail(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }

    /// Fails until the third attempt, so a retry has something to recover from.
    fn flaky(&self) -> Result<Receipt, GatewayError> {
        let attempt = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if self.failing.load(Ordering::SeqCst) || attempt < 3 {
            Err(GatewayError::Failed("the gateway is unhappy".to_owned()))
        } else {
            Ok(Receipt::charged(attempt))
        }
    }

    /// Charges a card.
    ///
    /// `#[retry]` is written **below** `#[circuit_breaker]`, so it ends up
    /// outermost and the breaker observes every attempt — which is what section
    /// 2 prints.
    #[circuit_breaker(self.breaker)]
    #[retry(self.retry)]
    async fn charge(&self) -> Result<Receipt, GatewayError> {
        self.flaky()
    }

    /// Queues the charge when the breaker is open.
    ///
    /// The `fallback` is the degradation path: a refusal is answered with a
    /// deferred receipt instead of an error. This method deliberately carries no
    /// retry, so section 3 shows the breaker on its own.
    #[circuit_breaker(self.breaker, fallback = Ok(Receipt::deferred()))]
    async fn charge_or_defer(&self) -> Result<Receipt, GatewayError> {
        self.flaky()
    }

    /// Signs a user in, inside a quota.
    #[rate_limiter(self.limiter)]
    async fn login(&self) -> Result<u32, GatewayError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(1)
    }

    /// Signs a user in as a guest when the quota is spent.
    #[rate_limiter(self.limiter, fallback = Ok(GUEST_SESSION))]
    async fn login_or_guest(&self) -> Result<u32, GatewayError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(1)
    }

    /// Searches, with a fifty millisecond budget.
    #[time_limiter(self.time_limiter)]
    async fn search(&self) -> Result<u32, GatewayError> {
        tokio::time::sleep(Duration::from_millis(500)).await;
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(1)
    }

    /// Renders a report, tracking how many run at once.
    #[bulkhead(self.bulkhead)]
    async fn report(&self) -> Result<u32, GatewayError> {
        let concurrent = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(concurrent, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(10)).await;
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(1)
    }
}

#[tokio::main]
async fn main() {
    NextWebApplication::<ConfiguredApplication>::default()
        .run()
        .await
}
