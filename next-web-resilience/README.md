# next-web-resilience

Resilience patterns for Next Web applications, in the shape Resilience4j gives
them.

Every call to an external service can go wrong in five different ways, and each
one needs a different answer. `next-web-resilience` provides one module per
answer, all sharing the same configuration style, the same event streams and the
same metrics snapshots:

| Pattern | Module | Question it answers | Typical use |
|---|---|---|---|
| Circuit breaker | `circuitbreaker` | Is the dependency still healthy? | A payment gateway that started timing out |
| Rate limiter | `ratelimiter` | Are we calling it too often? | A public API or login endpoint that must not be abused |
| Bulkhead | `bulkhead` | Are we using too much of our own capacity? | A slow search backend that must not consume every task slot |
| Retry | `retry` | Should we try again? | Network jitter and other transient faults |
| Time limiter | `timelimiter` | Is it taking too long? | A dependency that may block a task indefinitely |

It is a port of the [Resilience4j](https://resilience4j.readme.io/) modules of
the same names. Defaults, configuration names and state semantics follow the
Java originals, so the behaviour of a pattern is documented by the library it
came from.

## Features

- **One configuration style.** Every pattern has an `XxxConfig` with a builder,
  documented Resilience4j defaults, and validation at configuration time.
- **Shared instances through a registry.** `ResilienceRegistry` hands out one
  instance per resource name, so every caller observes the same counters. A
  pattern built per request could never measure anything.
- **Events and metrics.** Each pattern exposes `snapshot()` for its counters and
  `event_stream()` for a Tokio broadcast channel of what it did. Publishing never
  blocks the guarded call and costs nothing when nobody subscribes.
- **A composed pipeline.** `Resilience` applies all five patterns to one call in
  the order Resilience4j recommends:
  `retry → circuit breaker → rate limiter → time limiter → bulkhead → call`.
- **A pluggable budget for the rate limiter.** The limiter holds no budget
  itself: it asks a `RateLimiterStore`. The default keeps the budget in the
  process, and implementing the trait puts it in a shared service such as Redis
  so one quota covers every instance. The seam is deliberately a single
  indivisible operation, not a `get`/`set` pair — see
  [Sharing one budget across instances](#sharing-one-budget-across-instances).
- **Attribute macros.** `#[circuit_breaker]`, `#[rate_limiter]`, `#[bulkhead]`,
  `#[time_limiter]` and `#[retry]` wrap a method's body, so a guard is declared
  where the guarded work is written rather than at every call site. They stack,
  they leave the signature and the other attributes alone, and each takes a
  `fallback` for the degradation path.
- **Deterministic in tests.** The time-aware patterns read a `Clock`, so a test
  can install a `ManualClock` and advance time instead of sleeping. The
  async waiting paths use Tokio timers and work with `start_paused = true`.
- **Async first, no blocking locks.** No lock is held across an `.await`; no
  `unsafe`, no `unwrap`, no `println!`.

## Installation

```toml
[dependencies]
next-web-resilience = "0.1"
```

Or, when using the framework crate, enable the optional integration:

```toml
[dependencies]
next-web = { version = "0.2", features = ["enable-resilience"] }
```

which re-exports the crate as `next_web::resilience`.

The attribute macros are on by default. For a runtime-only build — or one that
must not pull in a procedural macro crate — turn them off:

```toml
[dependencies]
next-web-resilience = { version = "0.1", default-features = false }
```

## Usage

### One pattern at a time

```rust
use std::time::Duration;
use next_web_resilience::circuitbreaker::{CircuitBreaker, CircuitBreakerConfig};

let breaker = CircuitBreaker::new(
    "payments",
    CircuitBreakerConfig::builder()
        // Open once half of the last 20 calls failed, but not before 10 calls
        // have been made.
        .sliding_window_size(20)
        .minimum_number_of_calls(10)
        .failure_rate_threshold(50.0)
        // Stay open for 30 seconds, then let 3 calls probe the gateway.
        .wait_duration_in_open_state(Duration::from_secs(30))
        .permitted_number_of_calls_in_half_open_state(3)
        .build(),
);

let result = breaker
    .call(|| async { charge_card().await })
    .await;
```

The same shape works for the other four: build a config, build the pattern, then
either use its `call` helper or drive it by hand through `acquire_permission`,
`on_success` and `on_error`.

### Guarding a method with an attribute

The re-exported attribute macros wrap a method's body, so the guard sits next to
the code it protects instead of around every call site:

```rust
use std::sync::Arc;
use next_web_resilience::{
    circuit_breaker, retry,
    circuitbreaker::{CircuitBreaker, CircuitBreakerConfig},
    retry::{Retry, RetryConfig},
};

struct Payments {
    breaker: Arc<CircuitBreaker>,
    retry: Arc<Retry>,
}

impl Payments {
    // The attribute written *lowest* ends up outermost, so this retry wraps the
    // breaker — and the breaker therefore sees every attempt the retry makes.
    #[circuit_breaker(self.breaker)]
    #[retry(self.retry)]
    async fn charge(&self, amount: u64) -> Result<u64, PaymentError> {
        self.gateway.charge(amount).await
    }
}
```

Two things worth knowing before reaching for them:

- `#[rate_limiter]`, `#[bulkhead]` and `#[time_limiter]` guard `async fn` only —
  each either awaits something or drops the future it was given, and neither is
  possible in a synchronous body. The macro refuses with that explanation rather
  than silently guarding nothing, and `#[circuit_breaker]` and `#[retry]` guard
  either kind.
- Without a `fallback`, a rejection is converted into the method's own error with
  `Into::into`, so that error needs a `From` implementation for the rejection
  type. `fallback = <expression>` replaces that conversion, which is the
  degradation path: serve a cached value, a default, a cheaper answer.

```rust
// The named form resolves the instance from the process registry, which is what
// a method with nothing to point at needs.
#[circuit_breaker(name = "payments", fallback = Ok(Receipt::deferred()))]
async fn charge(&self) -> Result<Receipt, PaymentError> {
    self.gateway.charge().await
}
```

The `next-web-resilience-macros` crate documents the whole grammar, the stacking
order and the one case that does not work — a method inside an `#[async_trait]`
impl, which that attribute rewrites before the guard can see it.

### Sharing one instance

```rust
use std::sync::Arc;
use next_web_resilience::ratelimiter::RateLimiterConfig;
use next_web_resilience::registry::ResilienceRegistry;

async fn example() {
    let registry = Arc::new(ResilienceRegistry::new());

    // Created on first use, shared from then on: 100 calls per minute, waiting
    // up to 500 milliseconds for a permission before refusing.
    let limiter = registry.rate_limiter("login", || {
        RateLimiterConfig::builder()
            .limit_for_period(100)
            .limit_refresh_period(std::time::Duration::from_secs(60))
            .timeout_duration(std::time::Duration::from_millis(500))
            .build()
    });

    assert!(limiter.acquire_permission().await.is_ok());
}
```

### Sharing one budget across instances

A rate limiter keeps its budget in a `RateLimiterStore`. The default store lives
in the process, so several instances behind a load balancer each hand out the
full quota and the real rate is the quota multiplied by the number of instances.
To make one quota cover them all, put the budget in a store they share by
implementing `RateLimiterStore` for it.

The store below is a sketch — only the shape matters, and the hard part is the
`try_acquire` body:

```rust
use std::sync::Arc;
use next_web_resilience::async_trait;
use next_web_resilience::ratelimiter::{
    PermissionRequest, RateLimiter, RateLimiterConfig, RateLimiterStore, Refusal,
};
use next_web_resilience::registry::ResilienceRegistry;

/// Holds the budget on a shared server, such as Redis.
#[derive(Debug)]
struct RedisStore {
    // A connection pool.
}

#[async_trait]
impl RateLimiterStore for RedisStore {
    async fn try_acquire(&self, request: PermissionRequest<'_>) -> Result<(), Refusal> {
        // Run one server side script, keyed on `request.name`, that increments
        // the window counter and answers either "permitted" or "wait this long".
        //
        // It has to be a single indivisible step. Reading the counter and then
        // writing it back would let two instances each believe they were the
        // only caller, which is exactly the bug a shared store is meant to fix.
    }
}

fn build(store: Arc<dyn RateLimiterStore>) {
    let limiter = RateLimiter::with_store(
        "login",
        RateLimiterConfig::builder().limit_for_period(100).build(),
        store,
    );

    // A limiter built here still has to reach the composed pipeline.
    let registry = ResilienceRegistry::new();
    registry.register_rate_limiter("login", Arc::new(limiter));
}
```

Two things `RateLimiterStore` deliberately does not ask of an implementation:
it never sleeps — waiting is the limiter's job, driven by the `Refusal` the
store returns — and it never has to trust the limiter's clock, so a store in
another process can use its own (Redis `TIME`, or the script's expiry) and
instances do not have to stay in step. The full contract is in the
`rate_limiter_store` module documentation.

### Composing several patterns

```rust
use std::sync::Arc;
use std::time::Duration;
use next_web_resilience::bulkhead::BulkheadConfig;
use next_web_resilience::circuitbreaker::CircuitBreakerConfig;
use next_web_resilience::decorator::Resilience;
use next_web_resilience::registry::ResilienceRegistry;
use next_web_resilience::retry::RetryConfig;
use next_web_resilience::timelimiter::TimeLimiterConfig;

let registry = ResilienceRegistry::new();

registry.circuit_breaker("search", || {
    CircuitBreakerConfig::builder()
        .sliding_window_size(20)
        .minimum_number_of_calls(10)
        .failure_rate_threshold(50.0)
        .build()
});
registry.retry("search", || {
    RetryConfig::builder().max_attempts(3).build()
});
registry.time_limiter("search", || {
    TimeLimiterConfig::builder()
        .timeout_duration(Duration::from_millis(800))
        .build()
});
registry.bulkhead("search", || {
    BulkheadConfig::builder().max_concurrent_calls(16).build()
});

let resilience = Resilience::from_registry(&registry, "search");

let hits = resilience
    .execute(|| async { search("resilience patterns").await })
    .await;
```

The registrations above produce a pipeline of exactly those four patterns. Only
a failure of the guarded operation is retried: a rejection from the circuit
breaker, the rate limiter or the bulkhead ends the call at once, because those
patterns refuse a call precisely when making it again cannot help.

### See it run

The crate ships a runnable demonstration that exercises all five patterns and
prints what each one observed:

```text
cargo run -p next-web-resilience --example resilience_demo
```

It shows a retry recovering a flaky gateway, a breaker taking a dead dependency
out of rotation without calling it again, a slow call being cut off, a quota
being enforced, and a bulkhead capping concurrency at its limit.

## Modules

| Module | Contents |
|---|---|
| `circuitbreaker` | `CircuitBreaker`, `CircuitBreakerConfig`, `CircuitBreakerState`, `CircuitBreakerError`, `CircuitBreakerEvent`, `CircuitBreakerSnapshot` |
| `ratelimiter` | `RateLimiter`, `RateLimiterConfig`, `RateLimiterError`, `RateLimiterEvent`, `RateLimiterSnapshot`, `RateLimiterStore`, `InMemoryRateLimiterStore`, `PermissionRequest`, `Refusal` |
| `bulkhead` | `Bulkhead`, `BulkheadPermit`, `BulkheadConfig`, `BulkheadError`, `BulkheadEvent`, `BulkheadSnapshot` |
| `retry` | `Retry`, `RetryConfig`, `IntervalFunction`, `ExponentialIntervalFunction`, `FixedIntervalFunction`, `RetryError`, `RetryEvent`, `RetrySnapshot` |
| `timelimiter` | `TimeLimiter`, `TimeLimiterConfig`, `TimeLimiterError`, `TimeLimiterEvent`, `TimeLimiterSnapshot` |
| `registry` | `ResilienceRegistry` |
| `decorator` | `Resilience`, `ResilienceBuilder` |
| `clock` | `Clock`, `SystemClock`, `ManualClock` |
| `error` | `ResilienceError` and the individual rejection errors |

The attribute macros live in `next-web-resilience-macros`, a procedural macro
crate the default `macros` feature re-exports from the root of this one. They
share the macro namespace with the pattern modules, so `retry` names both the
module and the attribute.

## Notes and limitations

- **Retry only what is safe to run twice.** The retry module repeats the guarded
  operation; it cannot know whether that operation is idempotent. Retrying a
  call that already moved money or sent a message will do it again.
- **The rate limiter is per process unless you say otherwise.** The default
  store keeps the budget in one process's memory, so several instances behind a
  load balancer each get the full quota. Implement `RateLimiterStore` against a
  shared service to make one quota cover them all. While a shared store is in
  use, `snapshot().available_permissions` reports `None`: a store in another
  process cannot answer without a round trip, and reporting "unknown" as "nothing
  left" would be worse than admitting it.
- **The other four patterns are local, on purpose.** A bulkhead limits *this*
  process's concurrency, and a time limiter and a retry are local control flow,
  so there is nothing to share. A circuit breaker could be shared, but a correct
  one needs the sliding window *and* the state updated atomically on the server;
  a `get`/`set` seam would compute the wrong answer under concurrency, so no
  trait is offered for it rather than one that looks usable and is not.
- **The defaults are Resilience4j's.** A circuit breaker built with
  `CircuitBreakerConfig::default()` needs 100 calls in its window before it will
  open; lower `minimum_number_of_calls` and `sliding_window_size` for a
  low-traffic dependency.
- **Events are best-effort.** The event channel drops events for a subscriber
  that falls behind, so that instrumentation can never slow down the calls it
  observes.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
