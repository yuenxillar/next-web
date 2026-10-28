//! A bounded registry of retry metrics rendered in the Prometheus text format.
//!
//! The registry is deliberately dependency free: a fixed histogram, a handful of counters and a
//! `HashMap` guarded by one mutex, all updated synchronously. A name that is already known costs no
//! allocation; the exposition format is produced on demand, when a metrics endpoint is scraped, and
//! the counters are copied out of the lock first so a scrape never stalls a retry that is recording.

use std::{collections::HashMap, sync::Mutex, time::Duration};

use serde::Serialize;

use crate::support::metrics_retry_listener::{RetryMetricsRecorder, RetryObservation};

/// Bucket boundaries of the block duration histogram, in milliseconds.
const BUCKETS: [u64; 13] = [
    5, 10, 25, 50, 100, 250, 500, 1_000, 2_500, 5_000, 10_000, 30_000, 60_000,
];

/// The name every extra series is folded into once the registry is full.
const OTHER: &str = "other";

/// How a retry block ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryOutcome {
    /// The callback succeeded.
    Success,
    /// Every attempt failed and the recovery callback produced the value.
    Recovered,
    /// Every attempt failed and no recovery was available.
    Exhausted,
    /// The block ended with an error the caller sees, without using up its attempts: a stateful
    /// retry that will be presented again, a vetoed retry, or an interrupted wait.
    Failed,
    /// The block did not finish: the callback panicked or the caller cancelled the operation.
    Aborted,
}

impl RetryOutcome {
    /// Every outcome, in the order the exposition renders them.
    const ALL: [RetryOutcome; 5] = [
        RetryOutcome::Aborted,
        RetryOutcome::Exhausted,
        RetryOutcome::Failed,
        RetryOutcome::Recovered,
        RetryOutcome::Success,
    ];

    /// Returns the label value used in the exposition format.
    pub fn as_str(self) -> &'static str {
        match self {
            RetryOutcome::Success => "success",
            RetryOutcome::Recovered => "recovered",
            RetryOutcome::Exhausted => "exhausted",
            RetryOutcome::Failed => "failed",
            RetryOutcome::Aborted => "aborted",
        }
    }

    /// Returns the index of this outcome in the per-name counters.
    fn index(self) -> usize {
        match self {
            RetryOutcome::Aborted => 0,
            RetryOutcome::Exhausted => 1,
            RetryOutcome::Failed => 2,
            RetryOutcome::Recovered => 3,
            RetryOutcome::Success => 4,
        }
    }
}

/// One finished block count per name and outcome.
#[derive(Clone, Debug, Serialize)]
pub struct BlockSnapshot {
    /// The reporting name of the retry block.
    pub name: String,
    /// How the blocks ended.
    pub outcome: RetryOutcome,
    /// How many blocks ended that way.
    pub count: u64,
}

/// The per-name totals of a registry.
#[derive(Clone, Debug, Serialize)]
pub struct NameSnapshot {
    /// The reporting name of the retry block.
    pub name: String,
    /// The time spent in this name's blocks, in microseconds.
    ///
    /// Microseconds rather than milliseconds: a block that succeeds on its first attempt often
    /// takes less than a millisecond, and rounding that to zero makes the histogram useless for the
    /// most common case.
    pub total_micros: u64,
    /// The failed attempts recorded for this name.
    pub attempt_errors: u64,
}

/// A point-in-time copy of the registry, serialisable for a JSON endpoint.
#[derive(Clone, Debug, Default, Serialize)]
pub struct RetryMetricsSnapshot {
    /// Block counts, sorted by name and then outcome.
    pub blocks: Vec<BlockSnapshot>,
    /// Per-name totals, sorted by name.
    pub names: Vec<NameSnapshot>,
}

/// The counters of one name.
#[derive(Clone, Default)]
struct NameCounters {
    blocks: [u64; 5],
    total_micros: u64,
    buckets: [u64; BUCKETS.len() + 1],
    attempt_errors: u64,
}

/// Records retry metrics per reporting name and renders them for Prometheus.
///
/// The number of distinct names is bounded by [`RetryMetrics::MAX_NAMES`], `other` included;
/// beyond that every new name is folded into `other`, so a label built from an item id by mistake
/// degrades into one bucket instead of an unbounded map.
///
/// There is no retries counter on purpose: the retry count of a context is cumulative for a
/// stateful retry, so summing it across invocations would over-count (three one-attempt invocations
/// would report 1 + 2 + 3). Retry work is read from `next_web_retry_attempt_errors_total` against
/// `next_web_retry_blocks_total` instead.
#[derive(Default)]
pub struct RetryMetrics {
    names: Mutex<HashMap<Box<str>, NameCounters>>,
}

impl RetryMetrics {
    /// The number of distinct names the registry keeps, `other` included.
    pub const MAX_NAMES: usize = 256;

    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one finished block.
    ///
    /// The elapsed time is stored in microseconds so a sub-millisecond block stays visible in
    /// `_sum`; the histogram buckets themselves are in milliseconds and stay coarse on purpose.
    pub fn observe_block(&self, name: &str, outcome: RetryOutcome, elapsed: Duration) {
        let micros = u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX);
        let millis = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        let slot = outcome.index();

        let mut names = self.lock();
        with_counters(&mut names, name, |counters| {
            counters.blocks[slot] = counters.blocks[slot].saturating_add(1);
            counters.total_micros = counters.total_micros.saturating_add(micros);

            let bucket = BUCKETS
                .iter()
                .position(|bound| millis <= *bound)
                .unwrap_or(BUCKETS.len());
            counters.buckets[bucket] = counters.buckets[bucket].saturating_add(1);
        });
    }

    /// Records one failed attempt.
    pub fn observe_attempt_error(&self, name: &str) {
        let mut names = self.lock();
        with_counters(&mut names, name, |counters| {
            counters.attempt_errors = counters.attempt_errors.saturating_add(1);
        });
    }

    /// Renders the Prometheus text exposition format.
    ///
    /// The counters are copied out of the lock before anything is formatted, so a scrape of a large
    /// registry cannot delay a retry that is recording an observation.
    pub fn render_prometheus(&self) -> String {
        let entries = {
            let names = self.lock();
            copy_sorted(&names)
        };

        let mut out = String::with_capacity(1_024);

        out.push_str(
            "# HELP next_web_retry_blocks_total Retry blocks that finished, by outcome.\n",
        );
        out.push_str("# TYPE next_web_retry_blocks_total counter\n");
        for (name, counters) in &entries {
            let label = escape(name);
            for outcome in RetryOutcome::ALL {
                let count = counters.blocks[outcome.index()];
                if count == 0 {
                    continue;
                }
                out.push_str(&format!(
                    "next_web_retry_blocks_total{{name=\"{label}\",outcome=\"{}\"}} {count}\n",
                    outcome.as_str()
                ));
            }
        }

        out.push_str(
            "# HELP next_web_retry_attempt_errors_total Failed attempts, by retry name.\n",
        );
        out.push_str("# TYPE next_web_retry_attempt_errors_total counter\n");
        for (name, counters) in &entries {
            out.push_str(&format!(
                "next_web_retry_attempt_errors_total{{name=\"{}\"}} {}\n",
                escape(name),
                counters.attempt_errors
            ));
        }

        out.push_str("# HELP next_web_retry_block_duration_seconds Time spent in a retry block.\n");
        out.push_str("# TYPE next_web_retry_block_duration_seconds histogram\n");
        for (name, counters) in &entries {
            let total = counters
                .blocks
                .iter()
                .copied()
                .fold(0_u64, u64::saturating_add);
            if total == 0 {
                continue;
            }

            let label = escape(name);
            let mut cumulative = 0_u64;
            for (index, bound) in BUCKETS.iter().enumerate() {
                cumulative = cumulative.saturating_add(counters.buckets[index]);
                out.push_str(&format!(
                    "next_web_retry_block_duration_seconds_bucket{{name=\"{label}\",le=\"{}\"}} {cumulative}\n",
                    seconds(*bound)
                ));
            }
            cumulative = cumulative.saturating_add(counters.buckets[BUCKETS.len()]);
            out.push_str(&format!(
                "next_web_retry_block_duration_seconds_bucket{{name=\"{label}\",le=\"+Inf\"}} {cumulative}\n"
            ));
            out.push_str(&format!(
                "next_web_retry_block_duration_seconds_sum{{name=\"{label}\"}} {}\n",
                micros_as_seconds(counters.total_micros)
            ));
            out.push_str(&format!(
                "next_web_retry_block_duration_seconds_count{{name=\"{label}\"}} {total}\n"
            ));
        }

        out
    }

    /// Copies the counters into a serialisable snapshot.
    pub fn snapshot(&self) -> RetryMetricsSnapshot {
        let entries = {
            let names = self.lock();
            copy_sorted(&names)
        };

        let mut snapshot = RetryMetricsSnapshot::default();

        for (name, counters) in entries {
            for outcome in RetryOutcome::ALL {
                let count = counters.blocks[outcome.index()];
                if count == 0 {
                    continue;
                }
                snapshot.blocks.push(BlockSnapshot {
                    name: name.to_string(),
                    outcome,
                    count,
                });
            }

            snapshot.names.push(NameSnapshot {
                name: name.to_string(),
                total_micros: counters.total_micros,
                attempt_errors: counters.attempt_errors,
            });
        }

        snapshot
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<Box<str>, NameCounters>> {
        match self.names.lock() {
            Ok(names) => names,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl RetryMetricsRecorder for RetryMetrics {
    fn record_block(&self, observation: RetryObservation<'_>) {
        self.observe_block(observation.name, observation.outcome, observation.elapsed);
    }

    fn record_attempt_error(&self, name: &str) {
        self.observe_attempt_error(name);
    }
}

/// Applies `update` to the counters of `name`, folding names beyond the cap into `other`.
///
/// A known name takes the allocation-free path. One slot is reserved for `other`, so the map never
/// grows past [`RetryMetrics::MAX_NAMES`] entries. The update is a closure rather than a returned
/// reference because a reference would keep the map borrowed while the miss path inserts into it.
fn with_counters<R>(
    names: &mut HashMap<Box<str>, NameCounters>,
    name: &str,
    update: impl FnOnce(&mut NameCounters) -> R,
) -> R {
    let at_capacity = names.len() >= RetryMetrics::MAX_NAMES.saturating_sub(1);

    if let Some(existing) = names.get_mut(name) {
        return update(existing);
    }

    let key = if at_capacity { OTHER } else { name };
    update(names.entry(key.into()).or_default())
}

/// Copies the counters and sorts them by name, outside the lock.
fn copy_sorted(names: &HashMap<Box<str>, NameCounters>) -> Vec<(Box<str>, NameCounters)> {
    let mut entries: Vec<(Box<str>, NameCounters)> = names
        .iter()
        .map(|(name, counters)| (name.clone(), counters.clone()))
        .collect();
    entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    entries
}

/// Formats a millisecond count as the seconds value of a histogram bound.
fn seconds(millis: u64) -> String {
    format!("{:.3}", millis as f64 / 1_000.0)
}

/// Formats a microsecond count as the seconds value of a histogram sum.
fn micros_as_seconds(micros: u64) -> String {
    format!("{:.6}", micros as f64 / 1_000_000.0)
}

/// Escapes a label value for the exposition format.
fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            other => escaped.push(other),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{RetryMetrics, RetryOutcome};

    #[test]
    fn a_successful_block_renders_counters_and_a_histogram() {
        let metrics = RetryMetrics::new();
        metrics.observe_block("orders", RetryOutcome::Success, Duration::from_millis(12));

        let text = metrics.render_prometheus();

        assert!(
            text.contains("next_web_retry_blocks_total{name=\"orders\",outcome=\"success\"} 1")
        );
        assert!(text.contains("next_web_retry_attempt_errors_total{name=\"orders\"} 0"));
        assert!(text.contains(
            "next_web_retry_block_duration_seconds_bucket{name=\"orders\",le=\"0.010\"} 0"
        ));
        assert!(text.contains(
            "next_web_retry_block_duration_seconds_bucket{name=\"orders\",le=\"0.025\"} 1"
        ));
        assert!(text.contains("next_web_retry_block_duration_seconds_count{name=\"orders\"} 1"));
        assert!(
            text.contains("next_web_retry_block_duration_seconds_sum{name=\"orders\"} 0.012000")
        );
    }

    #[test]
    fn a_sub_millisecond_block_keeps_a_non_zero_sum() {
        let metrics = RetryMetrics::new();
        metrics.observe_block("orders", RetryOutcome::Success, Duration::from_micros(400));

        let text = metrics.render_prometheus();

        assert!(
            text.contains("next_web_retry_block_duration_seconds_sum{name=\"orders\"} 0.000400"),
            "a fast block must not round its duration to zero, got:\n{text}"
        );
        assert!(text.contains(
            "next_web_retry_block_duration_seconds_bucket{name=\"orders\",le=\"0.005\"} 1"
        ));
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
        metrics.observe_block("zeta", RetryOutcome::Success, Duration::from_millis(1));
        metrics.observe_block("alpha", RetryOutcome::Exhausted, Duration::from_millis(1));
        metrics.observe_block("beta\"q\\", RetryOutcome::Aborted, Duration::from_millis(1));
        metrics.observe_block("gamma\nq", RetryOutcome::Failed, Duration::from_millis(1));

        let text = metrics.render_prometheus();
        let alpha = text.find("name=\"alpha\"").expect("alpha must be rendered");
        let beta = text
            .find("name=\"beta\\\"q\\\\\"")
            .expect("the escaped quote and backslash must be rendered");
        let gamma = text
            .find("name=\"gamma\\nq\"")
            .expect("the escaped newline must be rendered");
        let zeta = text.find("name=\"zeta\"").expect("zeta must be rendered");

        assert!(alpha < beta && beta < gamma && gamma < zeta);
        assert!(
            !text.contains("gamma\nq"),
            "a newline inside a label must not reach the output"
        );
    }

    #[test]
    fn the_registry_folds_extra_names_into_other() {
        let metrics = RetryMetrics::new();
        for index in 0..RetryMetrics::MAX_NAMES + 10 {
            metrics.observe_block(
                &format!("name-{index}"),
                RetryOutcome::Success,
                Duration::from_millis(1),
            );
        }

        let text = metrics.render_prometheus();
        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.names.len(), RetryMetrics::MAX_NAMES);
        assert!(
            text.contains("next_web_retry_blocks_total{name=\"other\",outcome=\"success\"} 11"),
            "the names beyond the cap must fold into other, got:\n{text}"
        );
    }

    #[test]
    fn the_snapshot_is_serialisable() {
        let metrics = RetryMetrics::new();
        metrics.observe_block("orders", RetryOutcome::Exhausted, Duration::from_millis(7));
        metrics.observe_attempt_error("orders");

        let json = serde_json::to_string(&metrics.snapshot()).expect("the snapshot must serialise");

        assert!(json.contains("\"name\":\"orders\""));
        assert!(json.contains("\"outcome\":\"exhausted\""));
        assert!(json.contains("\"total_micros\":7000"));
        assert!(json.contains("\"attempt_errors\":1"));
    }

    #[test]
    fn the_registry_satisfies_the_recorder_trait() {
        use std::sync::Arc;

        use crate::support::metrics_retry_listener::{RetryMetricsRecorder, RetryObservation};

        let metrics = Arc::new(RetryMetrics::new());
        let recorder: Arc<dyn RetryMetricsRecorder> = metrics.clone();

        recorder.record_attempt_error("orders");
        recorder.record_block(RetryObservation {
            name: "orders",
            outcome: RetryOutcome::Recovered,
            retries: 1,
            elapsed: Duration::from_millis(3),
        });

        let text = metrics.render_prometheus();
        assert!(text.contains("next_web_retry_attempt_errors_total{name=\"orders\"} 1"));
        assert!(
            text.contains("next_web_retry_blocks_total{name=\"orders\",outcome=\"recovered\"} 1")
        );
    }
}
