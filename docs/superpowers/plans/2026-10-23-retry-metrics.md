# Retry Metrics Wiring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a retry's outcome, attempt failures and latency observable through a dependency-free registry that renders the Prometheus text format, and record a panic or a cancellation as `aborted` instead of `success`.

**Architecture:** Three layers, each independently testable. A registry (`RetryMetrics`) aggregates per-name counters and a fixed-bucket histogram under a bounded label cardinality and renders the exposition format. A recorder trait carries one finished block (name, outcome, retries, elapsed) and one failed attempt; a listener maps the retry lifecycle onto it. The template gains a completion flag so a `Drop`-driven cleanup can tell a finished block from a panicked or cancelled one.

**Tech Stack:** Rust 2024, `std` atomics + `Mutex` (no new runtime dependency), `serde` (already a dependency) for the JSON snapshot, `serde_json` as a dev-dependency for tests, Tokio for the tests, `axum` for the example binary in the `next-web-examples` package.

**Spec:** `next-web-retry/README.md` and the readiness criterion "metrics wiring is an availability dependency" agreed in this session. Java reference: `org.springframework.retry.support.MetricsRetryListener` (a Timer per block tagged `name`, `retry.count`, `exception`).

## Global Constraints

- No new dependency in `next-web-retry` except the dev-dependency `serde_json` (workspace version).
- Metric names are `next_web_retry_*`; label values for outcome are exactly `success`, `recovered`, `exhausted`, `aborted`.
- The registry must be bounded: at most `RetryMetrics::MAX_NAMES` (256) distinct names, further names fold into `other`.
- Recording happens on the retry path: no `await`, no I/O, no unbounded allocation; label strings are escaped at render time, not at record time.
- The crate has no external users yet (version 0.1.0), so changing the `RetryMetricsRecorder` trait is allowed; every call site in this repository must be updated in the same task.
- English doc comments on every public item; no `unwrap()` in non-test code; inline format args.
- Both this plan and `2026-10-23-linux-ci.md` append a section to `next-web-retry/README.md`. Finish
  the CI plan first, or apply both README sections in a single commit, whichever keeps the working
  tree moving; never let one overwrite the other.

## Review Focus

- **A block that succeeds on the first attempt** must count exactly one block with `outcome="success"`, `retries=0`, no attempt error, and a duration that is not assumed to be zero.
- **A stateful retry** produces one block per invocation (three calls on one item = three blocks). Dashboards must not read that as three items; the test pins the behaviour and the README states it. The registry's `retries_total` (Task 2 test) is the metric that shows the retry work inside those blocks.
- **A panic or a cancellation** must be recorded as `aborted`, never `success`; alerting depends on it.
- **A label containing `"`, `\` or a newline** must not corrupt the exposition format.
- **A high-cardinality name set** (a label built from an id by mistake) must not grow without bound; after 256 names everything folds into `other`.

---

### Task 1: Tell a finished block from a panicked or cancelled one

**Files:**
- Modify: `next-web-retry/src/retry_context.rs` (add the constant next to `EXHAUSTED`)
- Modify: `next-web-retry/src/support/retry_template.rs` (`ExecutionGuard`)
- Test: `next-web-retry/tests/production_paths.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `retry_context_constants::ABORTED: &str = "context.aborted"`, set on the context before the close hooks run when the block did not finish normally; `ExecutionGuard::mark_completed(&self)` (private to the template module).

- [ ] **Step 1: Write the failing tests**

Add to `next-web-retry/tests/production_paths.rs`:

```rust
struct AbortedProbe {
    seen: Arc<Mutex<Option<bool>>>,
}

impl RetryListener for AbortedProbe {
    fn close(&self, context: &dyn RetryContext, _error: Option<&dyn AnyError>) {
        if let Ok(mut seen) = self.seen.lock() {
            *seen = Some(context.flag(next_web_retry::retry_context::retry_context_constants::ABORTED));
        }
    }
}

#[tokio::test]
async fn a_completed_retry_is_not_marked_aborted() {
    let seen = Arc::new(Mutex::new(None));
    let template = RetryTemplate::builder()
        .max_attempts(1)
        .with_listener(AbortedProbe { seen: seen.clone() })
        .build();

    let _ = template.execute(succeeding(1_u16)).await;

    assert_eq!(seen.lock().ok().and_then(|seen| *seen), Some(false));
}

#[tokio::test]
async fn a_panicking_retry_is_marked_aborted() {
    let seen = Arc::new(Mutex::new(None));
    let template = RetryTemplate::builder()
        .max_attempts(1)
        .with_listener(AbortedProbe { seen: seen.clone() })
        .build();

    let handle = tokio::spawn(async move {
        let _ = template
            .execute(with_fn(|_| {
                Box::pin(async move {
                    panic!("the callback exploded");
                    #[allow(unreachable_code)]
                    Ok::<u16, RetryError>(0)
                })
            }))
            .await;
    });
    let _ = handle.await;

    assert_eq!(seen.lock().ok().and_then(|seen| *seen), Some(true));
}

#[tokio::test]
async fn a_cancelled_retry_is_marked_aborted() {
    let seen = Arc::new(Mutex::new(None));
    let template = RetryTemplate::builder()
        .max_attempts(1)
        .with_listener(AbortedProbe { seen: seen.clone() })
        .build();
    let started = Arc::new(tokio::sync::Notify::new());
    let started_in_call = started.clone();

    let handle = tokio::spawn(async move {
        let _ = template
            .execute(with_fn(move |_| {
                let started = started_in_call.clone();
                Box::pin(async move {
                    started.notify_one();
                    std::future::pending::<()>().await;
                    Ok::<u16, RetryError>(0)
                })
            }))
            .await;
    });
    started.notified().await;
    handle.abort();
    let _ = handle.await;

    assert_eq!(seen.lock().ok().and_then(|seen| *seen), Some(true));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p next-web-retry --test production_paths`
Expected: FAIL to compile with `no constant named ABORTED`. Add only the constant, run again, and
expect the two aborted tests to FAIL while the completed one passes — that split is the proof the
attribute is what the tests observe.

- [ ] **Step 3: Add the constant**

In `next-web-retry/src/retry_context.rs`, inside `pub mod retry_context_constants`, after `EXHAUSTED`:

```rust
    /// Set when a retry block did not finish normally, for example because the callback
    /// panicked or the caller cancelled the operation.
    pub const ABORTED: &str = "context.aborted";
```

- [ ] **Step 4: Teach the guard to mark an unfinished block**

In `next-web-retry/src/support/retry_template.rs`:

1. Add to `ExecutionGuard` a field `completed: AtomicBool` initialised to `false` in `new`.
2. Add:
   ```rust
   /// Records that the block reached one of its normal exits.
   fn mark_completed(&self) {
       self.completed.store(true, Ordering::Release);
   }
   ```
3. At the start of `run()`, before `self.template.close(...)`:
   ```rust
   if !self.completed.load(Ordering::Acquire) {
       self.context
           .set_attribute(retry_context_constants::ABORTED, true.into());
   }
   ```
4. Call `guard.mark_completed();` immediately before each of the five normal exits of the async
   block: the success `return Ok(result)`, the listener-veto `return Err(RetryError::terminated(...))`
   before the first attempt, the `register_throwable` failure `return Err(...)`, the stateful
   `return Err(error)`, and just before the final `self.handle_retry_exhausted(...)` expression (so
   the awaited result is the block value). A vetoed block is finished, not aborted: it must not be
   reported as `aborted`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p next-web-retry --test production_paths`
Expected: PASS, 22 tests (19 existing + 3 new).

- [ ] **Step 6: Commit**

```powershell
git add next-web-retry/src/retry_context.rs next-web-retry/src/support/retry_template.rs next-web-retry/tests/production_paths.rs
git commit -m "feat(retry): mark a block that panicked or was cancelled"
```

---

### Task 2: The bounded metrics registry

**Files:**
- Create: `next-web-retry/src/support/retry_metrics.rs`
- Modify: `next-web-retry/src/support/mod.rs` (declare and re-export)
- Modify: `next-web-retry/Cargo.toml` (dev-dependency `serde_json`)
- Test: the file's own `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: nothing.
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
  #[serde(rename_all = "snake_case")]
  pub enum RetryOutcome { Success, Recovered, Exhausted, Aborted }
  impl RetryOutcome { pub fn as_str(self) -> &'static str }

  #[derive(Clone, Debug, Default, serde::Serialize)]
  pub struct RetryMetricsSnapshot { pub blocks: Vec<BlockSnapshot>, pub names: Vec<NameSnapshot> }
  #[derive(Clone, Debug, serde::Serialize)]
  pub struct BlockSnapshot { pub name: String, pub outcome: RetryOutcome, pub count: u64 }
  #[derive(Clone, Debug, serde::Serialize)]
  pub struct NameSnapshot { pub name: String, pub retries: u64, pub total_millis: u64, pub attempt_errors: u64 }

  pub struct RetryMetrics { /* private */ }
  impl RetryMetrics {
      pub const MAX_NAMES: usize = 256;
      pub fn new() -> Self;
      pub fn observe_block(&self, name: &str, outcome: RetryOutcome, elapsed: Duration, retries: u16);
      pub fn observe_attempt_error(&self, name: &str);
      pub fn render_prometheus(&self) -> String;
      pub fn snapshot(&self) -> RetryMetricsSnapshot;
  }
  impl Default for RetryMetrics
  ```

- [ ] **Step 1: Write the failing tests**

Create the file with the types and empty method bodies returning `String::new()` / `RetryMetricsSnapshot::default()`, then add:

```rust
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{RetryMetrics, RetryOutcome};

    #[test]
    fn a_successful_block_renders_counters_and_a_histogram() {
        let metrics = RetryMetrics::new();
        metrics.observe_block("orders", RetryOutcome::Success, Duration::from_millis(12), 0);

        let text = metrics.render_prometheus();

        assert!(text.contains("next_web_retry_blocks_total{name=\"orders\",outcome=\"success\"} 1"));
        assert!(text.contains("next_web_retry_retries_total{name=\"orders\"} 0"));
        assert!(text.contains("next_web_retry_block_duration_seconds_bucket{name=\"orders\",le=\"0.010\"} 0"));
        assert!(text.contains("next_web_retry_block_duration_seconds_bucket{name=\"orders\",le=\"0.025\"} 1"));
        assert!(text.contains("next_web_retry_block_duration_seconds_count{name=\"orders\"} 1"));
        assert!(text.contains("next_web_retry_block_duration_seconds_sum{name=\"orders\"} 0.012"));
    }

    #[test]
    fn attempt_errors_are_counted_per_name() {
        let metrics = RetryMetrics::new();
        metrics.observe_attempt_error("orders");
        metrics.observe_attempt_error("orders");

        let text = metrics.render_prometheus();

        assert!(text.contains("next_web_retry_attempt_errors_total{name=\"orders\"} 2"));
    }

    #[test]
    fn the_exposition_is_sorted_and_escaped() {
        let metrics = RetryMetrics::new();
        metrics.observe_block("zeta", RetryOutcome::Success, Duration::from_millis(1), 0);
        metrics.observe_block("alpha", RetryOutcome::Exhausted, Duration::from_millis(1), 2);
        metrics.observe_block("beta\"q\\", RetryOutcome::Aborted, Duration::from_millis(1), 3);

        let text = metrics.render_prometheus();
        let alpha = text.find("name=\"alpha\"").expect("alpha must be rendered");
        let beta = text.find("name=\"beta\\\"q\\\\\"").expect("the escaped name must be rendered");
        let zeta = text.find("name=\"zeta\"").expect("zeta must be rendered");

        assert!(alpha < beta && beta < zeta);
    }

    #[test]
    fn the_registry_folds_extra_names_into_other() {
        let metrics = RetryMetrics::new();
        for index in 0..RetryMetrics::MAX_NAMES + 10 {
            metrics.observe_block(&format!("name-{index}"), RetryOutcome::Success, Duration::from_millis(1), 0);
        }

        let text = metrics.render_prometheus();

        assert!(text.contains("next_web_retry_blocks_total{name=\"other\",outcome=\"success\"} 10"));
        assert_eq!(text.matches("outcome=\"success\"} 1\n").count(), RetryMetrics::MAX_NAMES);
    }

    #[test]
    fn the_snapshot_is_serialisable() {
        let metrics = RetryMetrics::new();
        metrics.observe_block("orders", RetryOutcome::Exhausted, Duration::from_millis(7), 4);
        metrics.observe_attempt_error("orders");

        let json = serde_json::to_string(&metrics.snapshot()).expect("the snapshot must serialise");

        assert!(json.contains("\"name\":\"orders\""));
        assert!(json.contains("\"outcome\":\"exhausted\""));
        assert!(json.contains("\"retries\":4"));
    }}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p next-web-retry --lib retry_metrics`
Expected: FAIL on every assertion (the stub renders an empty string).

- [ ] **Step 3: Implement the registry**

Rules the implementation must follow:
- Buckets are millisecond bounds `[5, 10, 25, 50, 100, 250, 500, 1_000, 2_500, 5_000, 10_000, 30_000, 60_000]` stored in an array of `MAX_BUCKETS + 1` cumulative slots; render each bound as seconds with three decimals (`le="0.010"`), plus `_sum` and `_count`.
- Per name keep `blocks[outcome]`, `retries`, `total_millis`, `buckets`, plus a separate `attempt_errors` counter; a `Mutex<HashMap<Box<str>, NameCounters>>` is enough and matches the style of `next-web-gateway/src/metrics/mod.rs`.
- `MAX_NAMES` is enforced on first sight of a name: once the map holds 256 entries, every new name is recorded under `other`.
- Render in this order, each with `# HELP` and `# TYPE`: `next_web_retry_blocks_total` (counter), `next_web_retry_retries_total` (counter), `next_web_retry_attempt_errors_total` (counter), `next_web_retry_block_duration_seconds` (histogram). Sort names, and within a name sort outcome labels `aborted`, `exhausted`, `recovered`, `success`.
- Escape `\`, `"` and a newline in label values as `\\`, `\"`, `\n`.
- `snapshot()` returns `blocks` sorted by name and then outcome (one entry per name/outcome with a
  non-zero count), and `names` sorted by name carrying `retries`, `total_millis` and
  `attempt_errors` for that name.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p next-web-retry --lib retry_metrics`
Expected: PASS, 5 tests.

- [ ] **Step 5: Commit**

```powershell
git add next-web-retry/src/support/retry_metrics.rs next-web-retry/src/support/mod.rs next-web-retry/Cargo.toml Cargo.lock
git commit -m "feat(retry): add a bounded prometheus registry for retry metrics"
```

---

### Task 3: Reshape the recorder and map the lifecycle onto it

**Files:**
- Modify: `next-web-retry/src/support/metrics_retry_listener.rs` (replace the trait, the noop recorder and the listener's mapping)
- Modify: `next-web-retry/src/support/retry_metrics.rs` (implement the trait for `RetryMetrics`)
- Modify: `next-web-retry/src/support/mod.rs` (re-export the new items)
- Test: `metrics_retry_listener.rs`'s `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `RetryOutcome`, `RetryMetrics` from Task 2; `retry_context_constants::ABORTED` from Task 1.
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug)]
  pub struct RetryObservation<'a> {
      pub name: &'a str,
      pub outcome: RetryOutcome,
      pub retries: u16,
      pub elapsed: Duration,
  }

  pub trait RetryMetricsRecorder: Send + Sync {
      fn record_block(&self, observation: RetryObservation<'_>);
      fn record_attempt_error(&self, name: &str);
  }

  pub struct NoopRetryMetricsRecorder;                 // implements the trait, records nothing
  pub struct MetricsRetryListener { /* recorder */ }   // new(Arc<dyn RetryMetricsRecorder>)
  ```
  `MetricsRetryListener` keeps `resolve_metric_name`, `DEFAULT_METRIC_NAME` and `OPEN_TIMESTAMP_ATTRIBUTE`; `METRIC_NAME` is removed because the metric family names are now fixed in the registry.

- [ ] **Step 1: Write the failing tests**

Replace the existing test module's recorder with a capture recorder and add:

```rust
#[derive(Default)]
struct CapturingRecorder {
    blocks: Mutex<Vec<(String, RetryOutcome, u16, Duration)>>,
    errors: Mutex<Vec<String>>,
}

impl RetryMetricsRecorder for CapturingRecorder {
    fn record_block(&self, observation: RetryObservation<'_>) {
        self.blocks.lock().unwrap().push((
            observation.name.to_owned(),
            observation.outcome,
            observation.retries,
            observation.elapsed,
        ));
    }

    fn record_attempt_error(&self, name: &str) {
        self.errors.lock().unwrap().push(name.to_owned());
    }
}

#[test]
fn a_closed_context_records_a_success_block() {
    let recorder = Arc::new(CapturingRecorder::default());
    let listener = MetricsRetryListener::new(recorder.clone());
    let context = crate::context::retry_context_support::RetryContextSupport::new();
    context.set_attribute(retry_context_constants::NAME, "orders".into());
    context.set_attribute(OPEN_TIMESTAMP_ATTRIBUTE, 1_000_u64.into());
    context.set_attribute(retry_context_constants::CLOSED, true.into());

    listener.close(&context, None);

    let blocks = recorder.blocks.lock().unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].0, "orders");
    assert_eq!(blocks[0].1, RetryOutcome::Success);
}

#[test]
fn the_outcome_is_derived_from_the_context_flags() {
    // table: exhausted -> Exhausted, recovered -> Recovered, aborted -> Aborted,
    // closed only -> Success, neither flag nor closed -> Aborted
}

#[test]
fn a_failed_attempt_records_an_attempt_error_and_no_block() {
    // listener.on_error(...) then assert errors == ["orders"] and blocks is empty
}

#[test]
fn a_successful_attempt_records_nothing_until_close() {
    // listener.on_success(...) then assert blocks and errors are empty
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p next-web-retry --lib metrics_retry_listener`
Expected: FAIL to compile — `record_block` / `RetryObservation` do not exist yet.

- [ ] **Step 3: Implement the mapping**

- `open`: unchanged, store the millisecond timestamp in `OPEN_TIMESTAMP_ATTRIBUTE` (also store the name so a later close can resolve it even if the attribute is removed elsewhere — the existing `resolve_metric_name` reads the context lazily, which is enough; do not add a side map).
- `on_success`: no-op. A successful attempt is not a finished block; recording it here was the bug that made every success look like a recovery.
- `on_error`: `self.recorder.record_attempt_error(&resolve_metric_name(context))`.
- `close(context, error)`:
  1. resolve the name;
  2. compute `elapsed` from `OPEN_TIMESTAMP_ATTRIBUTE` (missing → `Duration::ZERO` + `warn!`), then remove the attribute;
  3. derive the outcome, in this order: `ABORTED` → `Aborted`, `RECOVERED` → `Recovered`, `EXHAUSTED` → `Exhausted`, `CLOSED` → `Success`, otherwise `Aborted`;
  4. `record_block(RetryObservation { name, outcome, retries: context.get_retry_count(), elapsed })`.

  The `error` argument is not used for the outcome: the flags are authoritative, and an error is present on a recovered block too.
- Implement `RetryMetricsRecorder for RetryMetrics`: `record_block` calls `observe_block`, `record_attempt_error` calls `observe_attempt_error`.
- Update `support/mod.rs` re-exports: `MetricsRetryListener`, `NoopRetryMetricsRecorder`, `RetryMetricsRecorder`, `RetryObservation`, `RetryMetrics`, `RetryMetricsSnapshot`, `BlockSnapshot`, `NameSnapshot`, `RetryOutcome`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p next-web-retry --lib` and then `cargo test -p next-web-retry`
Expected: PASS. Existing tests that asserted a recovery on `on_success` must be updated to the new mapping; every other test must stay green.

- [ ] **Step 5: Commit**

```powershell
git add next-web-retry/src/support/metrics_retry_listener.rs next-web-retry/src/support/retry_metrics.rs next-web-retry/src/support/mod.rs
git commit -m "feat(retry): record one observation per finished retry block"
```

---

### Task 4: Wire it into a runnable endpoint and document the alerting

**Files:**
- Create: `examples/retry_metrics.rs`
- Modify: `examples/Cargo.toml` (add `[[bin]] name = "retry_metrics"`, `path = "retry_metrics.rs"`)
- Modify: `next-web-retry/README.md` (new "Metrics" section)
- Test: the example binary, exercised over HTTP in Step 3

**Interfaces:**
- Consumes: `RetryMetrics`, `MetricsRetryListener`, `RetryOutcome` from Tasks 2 and 3, imported
  from the `next_web_retry` crate directly (the `next-web-examples` package already depends on it;
  do not route through the `next-web` facade, which would require its `enable-retry` feature).
- Produces: nothing consumed by later tasks. The CI plan's `consumers` job gains
  `cargo check -p next-web-examples --bin retry_metrics --locked` (edit
  `.github/workflows/retry.yml` in the same commit if that workflow already exists).

- [ ] **Step 1: Write the example**

The binary must:
1. build `let metrics = Arc::new(RetryMetrics::new());`
2. build a template with `.with_listener(MetricsRetryListener::new(metrics.clone()))`, `max_attempts(3)` and `fixed_backoff(10)`;
3. serve two routes on `127.0.0.1:8099` with `axum`: `POST /charge` runs a retry whose callback fails on the first two attempts (an `AtomicU16` counter) and succeeds afterwards, returning `200`; `GET /metrics` returns `metrics.render_prometheus()` as `text/plain; version=0.0.4`;
4. log the bound address with `tracing` so the test in Step 3 knows the port.

- [ ] **Step 2: Check the example compiles and is registered**

Run:

```powershell
cargo check -p next-web-examples --bin retry_metrics
```

Expected: exit 0. If the bin is not registered, cargo reports "no bin target named `retry_metrics`" — then the `[[bin]]` entry is missing.

- [ ] **Step 3: Exercise the endpoint end to end**

Run the binary in the background, then:

```powershell
$job = Start-Job { cargo run -p next-web-examples --bin retry_metrics }
Start-Sleep -Seconds 8
Invoke-WebRequest -Method Post -Uri http://127.0.0.1:8099/charge
$text = (Invoke-WebRequest -Uri http://127.0.0.1:8099/metrics).Content
$text
Stop-Job $job; Remove-Job $job
```

Expected in the output: `next_web_retry_blocks_total{name="charge",outcome="success"} 1`, `next_web_retry_attempt_errors_total{name="charge"} 2`, and a `next_web_retry_block_duration_seconds_bucket` line for `le="10.000"` with a count of 1.

- [ ] **Step 4: Document the metrics and the alerts**

Append to `next-web-retry/README.md`:

```markdown
## Metrics

`MetricsRetryListener` (in `support`) reports one observation per finished retry block to a
`RetryMetricsRecorder`. The crate ships `RetryMetrics`, a dependency-free registry that renders
the Prometheus text format and can also be snapshotted as JSON:

| Metric | Type | Labels |
|--------|------|--------|
| `next_web_retry_blocks_total` | counter | `name`, `outcome` (`success`, `recovered`, `exhausted`, `aborted`) |
| `next_web_retry_retries_total` | counter | `name` |
| `next_web_retry_attempt_errors_total` | counter | `name` |
| `next_web_retry_block_duration_seconds` | histogram | `name` |

The label value is the callback's label, or `retry.unknown` when it has none. Keep labels static:
the registry folds names beyond 256 into `other`, but a label built from an item id still turns one
metric into a useless `other` bucket. `aborted` means the block did not finish — a panic or a
cancelled call — so it must never be read as success.

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
```

- [ ] **Step 5: Commit**

```powershell
git add examples/retry_metrics.rs examples/Cargo.toml next-web-retry/README.md
git commit -m "feat(examples): expose retry metrics over http"
```

---

## Self-Review

1. **Spec coverage:** an exporter exists (Task 2), the lifecycle is mapped without lying about panics (Tasks 1 and 3), and the wiring is demonstrated over HTTP (Task 4) — the three pieces the readiness note asked for.
2. **Step scan:** each step is one file, one command or one test; the outcome-derivation table is the only place a body is spelled out, because the precedence could reasonably be implemented in two different orders.
3. **Type consistency:** `RetryOutcome` variants (`Success`, `Recovered`, `Exhausted`, `Aborted`) are used identically in Tasks 2, 3 and 4; `RetryObservation` field names match between the trait and the test recorder; `RetryMetrics::MAX_NAMES` is a public constant used by its own test.
4. **Review Focus:** first-attempt success (Task 3 test, `retries = 0`), stateful multiplicity (Task 4 README paragraph + Task 3's retries test), panic/cancel (Task 1 tests), label escaping (Task 2 test), cardinality cap (Task 2 test).
5. **Proportion:** four tasks; the largest is the registry, which carries the file's own tests and no duplicated code from the gateway registry (only its style).

---

## Amendments made during execution

- RetryOutcome gained a fifth variant, Failed: a fourth label set could not distinguish a
  finished success from a block that handed an error back to its caller without using up its
  attempts (a stateful rethrow, a vetoed retry, an interrupted wait). The listener now derives the
  outcome from borted > recovered > exhausted > (terminated || error) > success, the template
  sets context.terminated on its two terminated exits, and the interrupted-wait exit marks the
  block completed instead of leaving it to look cancelled. Tests: _stateful_rethrow_is_recorded_as_failed,
  n_interrupted_back_off_is_recorded_as_failed, _vetoed_retry_is_recorded_as_failed, plus the
  listener's flag table.
- The retries counter was removed: 
ext_web_retry_retries_total and NameSnapshot::retries are
  superseded by 
ext_web_retry_attempt_errors_total against 
ext_web_retry_blocks_total. The
  retry count of a stateful context is cumulative, so summing it per block over-counts (three
  one-attempt invocations reported 1+2+3). RetryObservation::retries stays, documented as
  informational. Supersedes the Task 2 counter list, the Task 2 render order (which now renders
  blocks, attempt errors, duration) and the README metric table.
- The block duration is measured by the template, not the listener: ExecutionGuard owns an
  Instant, publishes etry_context_constants::ELAPSED (a boxed Duration) before the close
  hooks and removes it afterwards. This supersedes OPEN_TIMESTAMP_ATTRIBUTE, 
ow_millis, the
  listener's open hook and the wall-clock milliseconds in Task 3. Durations are accumulated in
  microseconds (Task 2 stored milliseconds), so a sub-millisecond block keeps a non-zero _sum.
- Every block flag is cleared when a cached context is rehydrated (CLOSED, EXHAUSTED,
  RECOVERED, ABORTED, TERMINATED), not only the first three: a cancelled or vetoed block left
  ABORTED/TERMINATED on a global-state context and poisoned every later block of that key.
- The registry reserves one slot for other, copies its counters out of the lock before
  formatting, takes an allocation-free fast path for a known name, always emits
  ttempt_errors_total (including  ) and escapes \r as well as \\, \" and \n.
- Tests added for the three superseding behaviours: n_aborted_circuit_breaker_block_does_not_poison_the_next_invocation,
  _vetoed_circuit_breaker_block_does_not_poison_the_next_invocation,
  _first_attempt_success_records_one_block_with_a_non_zero_duration,
  each_stateful_invocation_records_one_block, plus the registry's sub-millisecond, newline and
  cap tests.
