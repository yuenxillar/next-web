# Next Web Retry

Retry templates, policies and back-off strategies for Next Web applications.

If this feels familiar, that is intentional: it is a Rust port of the retry semantics of Spring
Retry, with the parts that only make sense on the JVM replaced by Rust-native equivalents.

## Quick start

```rust
use next_web_retry::{
    error::retry_error::RetryError,
    retry_callback::with_fn,
    retry_operations::RetryOperations,
    support::retry_template::RetryTemplate,
};

# async fn example() -> Result<(), RetryError> {
let template = RetryTemplate::builder()
    .max_attempts(3)
    .fixed_backoff(200)
    .not_retry_on(RetryError::custom("fatal"))
    .build();

let value = template
    .execute(with_fn(|context| {
        Box::pin(async move {
            // `context` carries the attempt count, the last error and your own attributes.
            if context.get_retry_count() < 1 {
                return Err(RetryError::custom("transient"));
            }
            Ok(42)
        })
    }))
    .await?;

assert_eq!(value, 42);
# Ok(())
# }
```

## What is in the crate

| Module | Contents |
|--------|----------|
| `error` | `RetryError` with a comparable `RetryErrorKind`, plus the specialised failures (exhausted, terminated, back-off interrupted, cache capacity) |
| `retry_context`, `context` | the context that travels through one retry block, and the default implementation |
| `classifier` | binary (by value), type and field based classification, and value classifiers, all with optional cause traversal |
| `policy` | max attempts, timeout, simple (per error), always, never, composite, per-error, expression, circuit breaker, and the retry context caches |
| `backoff` | none, fixed, uniform random, exponential, exponential with jitter, the sleeper abstraction, and a policy builder |
| `retry_listener`, `listener` | lifecycle listeners, including one that receives the intercepted call |
| `stats` | counters, rolling exponential averages, a name-keyed repository and a listener that fills it |
| `support` | `RetryTemplate`, `RetryTemplateBuilder`, `DefaultRetryState`, `Args`, the task-local current context, metrics reporting and offline simulation |
| `interceptor` | stateless, stateful and circuit-breaker interceptors over a Rust-native call description |

## Design notes

- **Async first.** Callbacks, recovery callbacks and back-off waits are `async`; a wait uses the
  Tokio timer and never blocks a worker thread.
- **Cleanup always runs.** Policies and listeners are closed by a `Drop` guard, so a retry that
  returns early, panics, or is dropped because its caller was cancelled still releases what it
  holds. That is the Rust equivalent of the `finally` block of the original implementation.
- **No JVM machinery.** Reflection-based method interception became an explicit
  `MethodInvocation` plus a closure, expression policies take a predicate-like `Expression`, and
  soft references became weak references.
- **Errors are one type.** Everything returns `RetryError`; `RetryError::kind()` tells a caller
  what failed without losing the original error, which stays downcastable.
- **Classification can use the Rust type.** [`ErrorTypeClassifier`](next-web-retry/src/classifier/error_type_classifier.rs)
  matches by concrete type and by fields, so a service that owns its error types never has to
  compare rendered messages.
- **Nested retries work.** The context of an enclosing retry is inherited as the parent of a
  nested one, and the current context is visible through a task-local during a callback.
- **Stateful retries resume.** A failed call rethrows and stores its context under a key, so the
  next call for the same item continues instead of starting over.

## Choosing which errors are retried

By value, when the configuration names a representative failure:

```rust
# use next_web_retry::{error::retry_error::RetryError, support::retry_template::RetryTemplate};
let template = RetryTemplate::builder()
    .max_attempts(3)
    .not_retry_on(RetryError::custom("validation failed"))
    .build();
```

By type and fields, when the service owns its error types:

```rust
# use next_web_retry::{error::retry_error::RetryError, support::retry_template::RetryTemplate};
# #[derive(Clone, Debug, PartialEq, Eq, Hash)]
# struct HttpError { status: u16 }
# impl std::fmt::Display for HttpError {
#     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "http {}", self.status) }
# }
# impl std::error::Error for HttpError {}
let template = RetryTemplate::builder()
    .max_attempts(3)
    .retry_on_error_if::<HttpError, _>(|error| error.status >= 500)
    .traversing_error_causes()
    .build();

let error = RetryError::Any(Box::new(HttpError { status: 503 }));
# let _ = (template, error);
```

The two styles are exclusive, and each is a white or a black list: `retry_on_*` makes everything
else non-retryable, `not_retry_on_*` makes everything else retryable. The same choice is available
without the builder through [`ErrorTypeClassifier`](next-web-retry/src/classifier/error_type_classifier.rs).

## Differences from the original

| Original | Here | Why |
|----------|------|-----|
| AOP `MethodInterceptor` | `MethodInvocation` + closure (`interceptor` module) | Rust has no reflection; the call is passed explicitly |
| SpEL `ExpressionRetryPolicy` | `Expression` trait implemented by closures | No expression engine is needed for a predicate over the context |
| `BinaryExceptionClassifier` by class | `ErrorTypeClassifier` by Rust type and fields | Rust types and their fields replace class hierarchies |
| SoftReference cache | `SoftReferenceMapRetryContextCache` over `Weak` | Weak references are the native "may disappear" handle |
| Thread-local `RetrySynchronizationManager` | Tokio task-local | A thread local cannot follow a task across an await point |
| `ThreadWaitSleeper` (blocking) | Tokio timer sleeper | Blocking a worker thread starves every other task |
| `finally` block | `Drop` guard | Runs on early return, panic and cancellation alike |
| MicroMeter `MetricsRetryListener` | `RetryMetricsRecorder` + `MetricsRetryListener` | The recorder is the integration seam; a transport is the application's choice |
| Blocking method calls | `async` callbacks and waits | The runtime is async end to end |
| Configurable `Log` | `tracing` | Logging is the framework's, not the template's, concern |


## Configuration is validated early

A misconfigured template panics at build time 鈥?for example selecting two retry policies, or
asking for an exponential back off whose maximum is below its initial interval. These are startup
errors; discovering them while serving traffic would be worse.

## Testing

```text
cargo test -p next-web-retry
```

Two suites back the module:

- `tests/retry_template_tests.rs` drives the happy paths end to end: retry until success,
  exhaustion, recovery, error white and black lists, timeout, custom back off, listener ordering,
  nested retries, stateful resume, the circuit breaker, statistics, and both interceptors.
- `tests/production_paths.rs` covers the failure paths that decide whether a retry layer is safe
  to run: the exact error that reaches the caller, recovery being skipped for non-recoverable
  errors, the stateful back-off schedule surviving between calls, a full cache, a cache entry that
  expires after it is read, concurrent stateful retries, the attempt budget published before the
  `open` hook, a circuit breaker reopening after its reset timeout, raw versus qualified state
  keys, forced refresh, and the exponential schedule being monotonic and capped.

Unit tests live next to the code they cover, so every policy, back-off policy, cache and helper is
also tested in isolation.

## Metrics

`MetricsRetryListener` (in `support`) reports one observation per finished retry block to a
`RetryMetricsRecorder`. The crate ships `RetryMetrics`, a dependency-free registry that renders the
Prometheus text format and can also be snapshotted as JSON:

| Metric | Type | Labels |
|--------|------|--------|
| `next_web_retry_blocks_total` | counter | `name`, `outcome` (`success`, `recovered`, `exhausted`, `failed`, `aborted`) |
| `next_web_retry_attempt_errors_total` | counter | `name` |
| `next_web_retry_block_duration_seconds` | histogram | `name` |

There is no retries counter: the retry count of a stateful context is cumulative, so summing it
across invocations would over-count. Read the retry work from the error rate against the block
rate instead (the second alert below).

The label value is the callback's label, or `retry.unknown` when it has none. Keep labels static:
the registry folds names beyond 256 into `other`, but a label built from an item id still turns one
metric into a useless `other` bucket. `failed` means the block handed an error back to its caller
without using up its attempts 鈥?a stateful retry that will be presented again, a vetoed retry, or an
interrupted wait. `aborted` means the block did not finish at all 鈥?a panic or a cancelled call 鈥?so
it must never be read as success.

Wiring is three lines, and
[`examples/retry_metrics.rs`](../examples/retry_metrics.rs) is a runnable end-to-end example:

```rust,ignore
let metrics = Arc::new(RetryMetrics::new());
let template = RetryTemplate::builder()
    .max_attempts(3)
    .with_listener(MetricsRetryListener::new(metrics.clone()))
    .build();
// GET /metrics -> metrics.render_prometheus()
```

Useful alerts:

```promql
# More than 5% of retry blocks exhaust their attempts.
sum(rate(next_web_retry_blocks_total{outcome="exhausted"}[5m]))
  / sum(rate(next_web_retry_blocks_total[5m])) > 0.05

# Retry amplification: more than three failed attempts per block.
sum(rate(next_web_retry_attempt_errors_total[5m]))
  / sum(rate(next_web_retry_blocks_total[5m])) > 3

# p99 block duration above 30s.
histogram_quantile(0.99, sum(rate(next_web_retry_block_duration_seconds_bucket[5m])) by (le, name)) > 30
```

A stateful retry produces one block per invocation: three attempts on one item are three blocks,
not one.

## Continuous integration

Every pull request runs `.github/workflows/retry.yml`, which gates this crate on Linux with the
toolchain the workspace declares (`rust-version = "1.98.1"`):

| Check | Command |
|-------|---------|
| Format | `cargo fmt -p next-web-retry --check` |
| Clippy | `cargo clippy -p next-web-retry --all-targets --no-deps -- -D warnings` |
| Test | `cargo test -p next-web-retry` |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc -p next-web-retry --no-deps` |
| Consumers | `cargo check -p next-web --features enable-retry`, `-p next-web-ai`, and the `retry` and `retry_metrics` example bins |

`--no-deps` keeps the clippy gate on this crate: the workspace still has pre-existing clippy
findings in `next-web-context` and `next-web-core`, and `-D warnings` would fail on them.

The commands do not pass `--locked`: this repository ignores `Cargo.lock` (`.gitignore`), so a
checkout has no lock file and `--locked` fails during resolution. Each job therefore resolves the
graph from the manifests; cargo's MSRV-aware resolver keeps the selection compatible with
`rust-version = "1.98.1"`. If reproducible CI is wanted later, commit `Cargo.lock` and add
`--locked` back to every command together.

The workflow runs on every pull request without a path filter, so the required checks always
report; pushes are path-filtered to avoid rebuilding this crate for unrelated changes. The `deny`
job is informational until the workspace's dependency findings are fixed.

Workspace-wide `clippy` and `rustfmt` are not enforced yet: `cargo fmt --all --check` and
`cargo clippy --workspace` still report pre-existing findings in `next-web-core` and
`next-web-ai`. Widening the gate is a follow-up, and this section must be updated when it happens.


