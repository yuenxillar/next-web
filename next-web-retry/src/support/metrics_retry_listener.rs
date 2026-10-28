//! Retry lifecycle metrics.
//!
//! The listener reports one observation per finished retry block — its name, how it ended, how
//! many retries it performed and how long it took — plus one observation per failed attempt to a
//! pluggable [`RetryMetricsRecorder`]. A successful attempt is deliberately not an observation:
//! only the block boundary knows the outcome, and reporting the attempt instead would count every
//! success as a recovery.
//!
//! The metric name is the reporting name of the retry context, the `NAME` attribute the caller
//! sets to identify the operation being retried. When no name is present,
//! [`DEFAULT_METRIC_NAME`] is used, so a recorder always receives a usable name.
//!
//! The block duration is measured by the template, not by this listener: one retry context can be
//! shared by concurrent blocks (a stateful cache hit, a circuit-breaker context), so a timestamp
//! stored on the context would be overwritten. The template publishes the elapsed time of the block
//! under [`retry_context_constants::ELAPSED`] just before the close hooks run, and removes it
//! afterwards.

use std::{any::Any, sync::Arc, time::Duration};

use next_web_core::any::AnyError;
use tracing::warn;

use crate::{
    retry_context::{RetryContext, retry_context_constants},
    retry_listener::RetryListener,
    support::retry_metrics::RetryOutcome,
};

/// Name used when the retry context carries no operation name.
pub const DEFAULT_METRIC_NAME: &str = "retry.unknown";

/// One finished retry block.
///
/// `retries` is the retry count of the context. For a stateful retry that count is cumulative
/// across invocations, so it is informational: a recorder that aggregates must not sum it.
#[derive(Clone, Copy, Debug)]
pub struct RetryObservation<'a> {
    /// The reporting name of the retry operation.
    pub name: &'a str,
    /// How the block ended.
    pub outcome: RetryOutcome,
    /// The context retry count at the end of the block.
    pub retries: u16,
    /// How long the block took, waiting included.
    pub elapsed: Duration,
}

/// Receives the observations of [`MetricsRetryListener`].
///
/// Implementations forward the observations to whatever metrics backend the application uses.
/// Every method is a plain synchronous call so recorders can stay cheap; an observation must never
/// block for long because it runs on the retry path.
pub trait RetryMetricsRecorder
where
    Self: Send + Sync,
{
    /// Records one finished retry block.
    fn record_block(&self, observation: RetryObservation<'_>);

    /// Records one failed attempt of the named operation.
    fn record_attempt_error(&self, name: &str);
}

/// A recorder that discards every observation.
///
/// It is the default of [`MetricsRetryListener`] and is useful when metrics are not collected but
/// the listener is wired unconditionally.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoopRetryMetricsRecorder;

impl RetryMetricsRecorder for NoopRetryMetricsRecorder {
    fn record_block(&self, _observation: RetryObservation<'_>) {}

    fn record_attempt_error(&self, _name: &str) {}
}

/// A [`RetryListener`] that turns retry lifecycle events into metrics.
///
/// The listener is stateless apart from the recorder it forwards to, so a single instance can be
/// shared by any number of retry operations. Everything it needs per block is published on the
/// context by the template.
#[derive(Clone)]
pub struct MetricsRetryListener {
    recorder: Arc<dyn RetryMetricsRecorder>,
}

impl MetricsRetryListener {
    /// Creates a listener that forwards to `recorder`.
    pub fn new(recorder: Arc<dyn RetryMetricsRecorder>) -> Self {
        Self { recorder }
    }

    /// Returns the recorder this listener forwards to.
    pub fn get_recorder(&self) -> &Arc<dyn RetryMetricsRecorder> {
        &self.recorder
    }
}

impl Default for MetricsRetryListener {
    fn default() -> Self {
        Self::new(Arc::new(NoopRetryMetricsRecorder))
    }
}

/// Returns the metric name of the operation described by `context`.
///
/// The name comes from the retry context reporting name, which is the `context.name` attribute of
/// the retry machinery; an absent or empty name falls back to [`DEFAULT_METRIC_NAME`].
pub fn resolve_metric_name(context: &dyn RetryContext) -> String {
    context
        .get_name()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| DEFAULT_METRIC_NAME.to_owned())
}

/// Derives how a block ended from the flags the template and the policies set on the context.
///
/// The flags are authoritative: an error is present on a recovered block too, so the error alone
/// cannot tell a recovery from an exhaustion. `had_error` only separates a finished success from a
/// block that handed an error back to its caller without using up its attempts; every template
/// exit either sets a flag or passes an error, so the unflagged branch is a defensive default.
fn outcome_of(context: &dyn RetryContext, had_error: bool) -> RetryOutcome {
    if context.flag(retry_context_constants::ABORTED) {
        return RetryOutcome::Aborted;
    }
    if context.flag(retry_context_constants::RECOVERED) {
        return RetryOutcome::Recovered;
    }
    if context.flag(retry_context_constants::EXHAUSTED) {
        return RetryOutcome::Exhausted;
    }
    if context.flag(retry_context_constants::TERMINATED) || had_error {
        return RetryOutcome::Failed;
    }

    RetryOutcome::Success
}

impl RetryListener for MetricsRetryListener {
    /// The template starts the clock of the block, so there is nothing to record when it opens.
    fn open(&self, _context: &dyn RetryContext) -> bool {
        true
    }

    /// Records the finished block, reading the duration the template published.
    fn close(&self, context: &dyn RetryContext, error: Option<&dyn AnyError>) {
        let name = resolve_metric_name(context);
        let elapsed = context
            .get_attribute(retry_context_constants::ELAPSED)
            .and_then(|value| value.as_ref_value::<Duration>().copied())
            .unwrap_or_else(|| {
                warn!(
                    "no block duration was published for the retry context; reporting zero elapsed time"
                );
                Duration::ZERO
            });

        self.recorder.record_block(RetryObservation {
            name: &name,
            outcome: outcome_of(context, error.is_some()),
            retries: context.get_retry_count(),
            elapsed,
        });
    }

    /// A successful attempt is not a finished block; the close hook records the outcome.
    fn on_success(&self, _context: &dyn RetryContext, _result: &dyn Any) {}

    fn on_error(&self, context: &dyn RetryContext, _error: &dyn AnyError) {
        self.recorder
            .record_attempt_error(&resolve_metric_name(context));
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use next_web_core::any::{AnyError, AnyValue};

    use super::{
        DEFAULT_METRIC_NAME, MetricsRetryListener, RetryMetricsRecorder, RetryObservation,
        resolve_metric_name,
    };
    use crate::{
        context::retry_context_support::RetryContextSupport,
        error::retry_error::RetryError,
        retry_context::{SyncAttributeAccessor, retry_context_constants},
        retry_listener::RetryListener,
        support::retry_metrics::RetryOutcome,
    };

    /// One recorded block, flattened so the assertions read like the exposition does.
    #[derive(Clone, Debug)]
    struct Captured {
        name: String,
        outcome: RetryOutcome,
        retries: u16,
        elapsed: Duration,
    }

    #[derive(Default)]
    struct CapturingRecorder {
        blocks: Mutex<Vec<Captured>>,
        errors: Mutex<Vec<String>>,
    }

    impl RetryMetricsRecorder for CapturingRecorder {
        fn record_block(&self, observation: RetryObservation<'_>) {
            if let Ok(mut blocks) = self.blocks.lock() {
                blocks.push(Captured {
                    name: observation.name.to_owned(),
                    outcome: observation.outcome,
                    retries: observation.retries,
                    elapsed: observation.elapsed,
                });
            }
        }

        fn record_attempt_error(&self, name: &str) {
            if let Ok(mut errors) = self.errors.lock() {
                errors.push(name.to_owned());
            }
        }
    }

    /// A context the way the template leaves it right before the close hooks run.
    fn named_context(name: &str, elapsed: Duration) -> RetryContextSupport {
        let context = RetryContextSupport::new();
        context.set_attribute(retry_context_constants::NAME, AnyValue::from(name));
        context.set_attribute(
            retry_context_constants::ELAPSED,
            AnyValue::BoxedValue(Box::new(elapsed)),
        );
        context
    }

    #[test]
    fn a_closed_context_records_a_success_block() {
        let recorder = Arc::new(CapturingRecorder::default());
        let listener = MetricsRetryListener::new(recorder.clone());
        let context = named_context("orders", Duration::from_micros(1_500));
        context.set_attribute(retry_context_constants::CLOSED, true.into());

        listener.close(&context, None);

        let blocks = recorder
            .blocks
            .lock()
            .map(|blocks| blocks.clone())
            .unwrap_or_default();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].name, "orders");
        assert_eq!(blocks[0].outcome, RetryOutcome::Success);
        assert_eq!(blocks[0].retries, 0);
        assert_eq!(blocks[0].elapsed, Duration::from_micros(1_500));
    }

    #[test]
    fn the_outcome_is_derived_from_the_context_flags() {
        let error = RetryError::custom("boom");
        let cases = [
            (
                vec![retry_context_constants::CLOSED],
                None,
                RetryOutcome::Success,
            ),
            (vec![], None, RetryOutcome::Success),
            (
                vec![
                    retry_context_constants::CLOSED,
                    retry_context_constants::EXHAUSTED,
                ],
                Some(&error),
                RetryOutcome::Exhausted,
            ),
            (
                vec![
                    retry_context_constants::CLOSED,
                    retry_context_constants::EXHAUSTED,
                    retry_context_constants::RECOVERED,
                ],
                Some(&error),
                RetryOutcome::Recovered,
            ),
            (
                vec![
                    retry_context_constants::CLOSED,
                    retry_context_constants::ABORTED,
                ],
                Some(&error),
                RetryOutcome::Aborted,
            ),
            (
                vec![
                    retry_context_constants::CLOSED,
                    retry_context_constants::TERMINATED,
                ],
                Some(&error),
                RetryOutcome::Failed,
            ),
            // A closed block that still hands an error to the caller: a stateful rethrow or an
            // interrupted wait.
            (
                vec![retry_context_constants::CLOSED],
                Some(&error),
                RetryOutcome::Failed,
            ),
        ];

        for (flags, error, expected) in cases {
            let recorder = Arc::new(CapturingRecorder::default());
            let listener = MetricsRetryListener::new(recorder.clone());
            let context = named_context("orders", Duration::from_millis(1));
            for flag in flags {
                context.set_attribute(flag, true.into());
            }

            listener.close(&context, error.map(|error| error as &dyn AnyError));

            let blocks = recorder
                .blocks
                .lock()
                .map(|blocks| blocks.clone())
                .unwrap_or_default();
            assert_eq!(blocks.len(), 1, "one block per close");
            assert_eq!(blocks[0].outcome, expected);
        }
    }

    #[test]
    fn a_failed_attempt_records_an_attempt_error_and_no_block() {
        let recorder = Arc::new(CapturingRecorder::default());
        let listener = MetricsRetryListener::new(recorder.clone());
        let context = named_context("orders", Duration::ZERO);

        listener.on_error(&context, &RetryError::custom("boom"));

        let blocks = recorder
            .blocks
            .lock()
            .map(|blocks| blocks.len())
            .unwrap_or_default();
        let errors = recorder
            .errors
            .lock()
            .map(|errors| errors.clone())
            .unwrap_or_default();
        assert_eq!(blocks, 0);
        assert_eq!(errors, vec!["orders".to_owned()]);
    }

    #[test]
    fn a_successful_attempt_records_nothing_until_close() {
        let recorder = Arc::new(CapturingRecorder::default());
        let listener = MetricsRetryListener::new(recorder.clone());
        let context = named_context("orders", Duration::ZERO);

        listener.on_success(&context, &42_u16);

        let blocks = recorder
            .blocks
            .lock()
            .map(|blocks| blocks.len())
            .unwrap_or_default();
        let errors = recorder
            .errors
            .lock()
            .map(|errors| errors.len())
            .unwrap_or_default();
        assert_eq!(
            blocks, 0,
            "a success is reported by close, not by on_success"
        );
        assert_eq!(errors, 0);
    }

    #[test]
    fn the_retry_count_is_reported_with_the_block() {
        let recorder = Arc::new(CapturingRecorder::default());
        let listener = MetricsRetryListener::new(recorder.clone());
        let context = named_context("orders", Duration::from_millis(4));
        context.register_error(&RetryError::custom("first"));
        context.register_error(&RetryError::custom("second"));
        context.register_error(&RetryError::custom("third"));
        context.set_attribute(retry_context_constants::EXHAUSTED, true.into());

        listener.close(&context, Some(&RetryError::custom("third")));

        let blocks = recorder
            .blocks
            .lock()
            .map(|blocks| blocks.clone())
            .unwrap_or_default();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].outcome, RetryOutcome::Exhausted);
        assert_eq!(blocks[0].retries, 3);
    }

    #[test]
    fn a_missing_duration_records_zero_elapsed() {
        let recorder = Arc::new(CapturingRecorder::default());
        let listener = MetricsRetryListener::new(recorder.clone());
        let context = named_context("orders", Duration::from_millis(1));
        context.remove_attribute(retry_context_constants::ELAPSED);
        context.set_attribute(retry_context_constants::CLOSED, true.into());

        listener.close(&context, None);

        let blocks = recorder
            .blocks
            .lock()
            .map(|blocks| blocks.clone())
            .unwrap_or_default();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].elapsed, Duration::ZERO);
    }

    #[test]
    fn the_metric_name_falls_back_when_the_context_has_no_name() {
        let context = RetryContextSupport::new();

        assert_eq!(resolve_metric_name(&context), DEFAULT_METRIC_NAME);
    }

    #[test]
    fn the_listener_reports_the_recorder_it_was_built_with() {
        let recorder = Arc::new(CapturingRecorder::default());
        let handle: Arc<dyn RetryMetricsRecorder> = recorder.clone();
        let listener = MetricsRetryListener::new(recorder);

        assert!(Arc::ptr_eq(listener.get_recorder(), &handle));
    }
}
