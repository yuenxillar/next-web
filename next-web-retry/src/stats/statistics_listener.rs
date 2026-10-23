//! Records retry-block outcomes into a statistics repository.

use std::sync::Arc;

use next_web_core::any::AnyError;

use crate::{
    retry_context::{RetryContext, retry_context_constants},
    retry_listener::RetryListener,
    stats::statistics_repository::StatisticsRepository,
};

/// Records retry-block outcomes into a [`StatisticsRepository`].
///
/// # Naming
///
/// The statistics a block belongs to are identified by the retry context's
/// reporting name ([`RetryContext::get_name`]), which the retry callback or the
/// code driving the retry sets. [`StatisticsListener::with_default_name`]
/// configures a fallback name for blocks that do not set one. When neither a
/// context name nor a default name is available the listener records nothing,
/// which keeps unrelated blocks out of the repository.
///
/// # Counting rules
///
/// The counters describe whole retry blocks, so the listener observes the
/// outcome at close time rather than at open time; it deliberately does not
/// override [`RetryListener::open`], which cannot know whether the block will
/// complete, abort, or be recovered.
///
/// * A closing block counts as **started** unless it was exhausted while its
///   state was kept globally: in that case the retry callback never ran. A
///   stateless retry enters the callback once per error without closing the
///   context, so [`RetryListener::on_error`] counts the start instead.
/// * A closing block counts as **recovered** when a recovery callback produced
///   a result, as **aborted** when the retry was exhausted, and as **completed**
///   when it closed without either condition. The order matters: a recovered
///   block is never also counted as aborted.
/// * Every error observed by [`RetryListener::on_error`] counts as an
///   **error**, whether or not another attempt follows.
///
/// # Diagnostic attributes
///
/// The reference behaviour also mirrors circuit-breaker attributes from the
/// retry context into the statistics object. Statistics here are plain counters
/// that own no attribute store, and this crate's circuit breaker keeps its state
/// in its own context, so nothing is mirrored: a reader inspects the circuit
/// breaker itself instead of a copy of its flags.
///
/// The listener holds only a repository handle, so the same instance can be
/// registered with several retry templates and they all report into one place.
pub struct StatisticsListener {
    repository: Arc<dyn StatisticsRepository>,
    default_name: Option<String>,
}

impl StatisticsListener {
    /// Creates a listener that names blocks from the retry context only.
    pub fn new(repository: Arc<dyn StatisticsRepository>) -> Self {
        Self {
            repository,
            default_name: None,
        }
    }

    /// Creates a listener that falls back to `name` for contexts that carry no
    /// reporting name.
    ///
    /// The context name wins when both are available, so one listener can serve
    /// several named blocks plus an unnamed default.
    pub fn with_default_name(
        repository: Arc<dyn StatisticsRepository>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            repository,
            default_name: Some(name.into()),
        }
    }

    /// Returns the repository this listener reports into.
    pub fn repository(&self) -> &Arc<dyn StatisticsRepository> {
        &self.repository
    }

    /// Returns the fallback name used when a context carries no name.
    pub fn default_name(&self) -> Option<&str> {
        self.default_name.as_deref()
    }

    /// Resolves the statistics name for `context`.
    ///
    /// A name attribute that is not a string is not a reporting name, so it is
    /// treated like a missing one and the fallback applies.
    fn statistics_name(&self, context: &dyn RetryContext) -> Option<String> {
        context.get_name().or_else(|| self.default_name.clone())
    }
}

impl RetryListener for StatisticsListener {
    fn close(&self, context: &dyn RetryContext, _error: Option<&dyn AnyError>) {
        let Some(name) = self.statistics_name(context) else {
            return;
        };

        let exhausted = context.has_attribute(retry_context_constants::EXHAUSTED);
        if !exhausted || context.has_attribute(retry_context_constants::GLOBAL_STATE) {
            self.repository.add_started(&name);
        }

        if context.has_attribute(retry_context_constants::RECOVERED) {
            self.repository.add_recovery(&name);
        } else if exhausted {
            self.repository.add_abort(&name);
        } else if context.has_attribute(retry_context_constants::CLOSED) {
            self.repository.add_complete(&name);
        }
    }

    fn on_error(&self, context: &dyn RetryContext, _error: &dyn AnyError) {
        let Some(name) = self.statistics_name(context) else {
            return;
        };

        // A stateless retry re-enters the callback once per error without
        // closing the context, so each error is also a fresh block start. A
        // stateful retry keeps the context, so its start is counted at close.
        if !context.has_attribute(retry_context_constants::STATE_KEY) {
            self.repository.add_started(&name);
        }
        self.repository.add_error(&name);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use next_web_core::any::AnyValue;

    use super::StatisticsListener;
    use crate::{
        context::retry_context_support::RetryContextSupport,
        retry_context::{SyncAttributeAccessor, retry_context_constants},
        retry_listener::RetryListener,
        stats::{
            default_statistics_repository::DefaultStatisticsRepository,
            statistics_repository::StatisticsRepository,
        },
    };

    /// A test error satisfying the bounds required by `AnyError`.
    #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    struct TestError;

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "test error")
        }
    }

    impl std::error::Error for TestError {}

    /// A listener wired to one repository, so a test can drive the listener and
    /// then inspect what was recorded.
    struct Fixture {
        listener: StatisticsListener,
        repository: Arc<DefaultStatisticsRepository>,
    }

    impl Fixture {
        fn new() -> Self {
            Self::with_listener(StatisticsListener::new)
        }

        fn with_default_name(name: &str) -> Self {
            Self::with_listener(|repository| {
                StatisticsListener::with_default_name(repository, name)
            })
        }

        fn with_listener(
            build: impl FnOnce(Arc<dyn StatisticsRepository>) -> StatisticsListener,
        ) -> Self {
            let repository = Arc::new(DefaultStatisticsRepository::new());
            let listener = build(repository.clone());
            Self {
                listener,
                repository,
            }
        }

        fn context(&self, name: Option<&str>) -> RetryContextSupport {
            let context = RetryContextSupport::new();
            if let Some(name) = name {
                context.set_attribute(retry_context_constants::NAME, AnyValue::from(name));
            }
            context
        }

        fn statistics(&self, name: &str) -> Arc<dyn crate::stats::MutableRetryStatistics> {
            self.repository
                .find_one(name)
                .expect("statistics should have been recorded")
        }
    }

    #[test]
    fn a_completed_block_counts_started_and_complete() {
        let fixture = Fixture::new();
        let context = fixture.context(Some("test"));
        context.set_attribute(retry_context_constants::CLOSED, AnyValue::Boolean(true));

        fixture.listener.close(&context, None);

        let statistics = fixture.statistics("test");
        assert_eq!(statistics.get_started_count(), 1);
        assert_eq!(statistics.get_complete_count(), 1);
        assert_eq!(statistics.get_error_count(), 0);
        assert_eq!(statistics.get_abort_count(), 0);
        assert_eq!(statistics.get_recovery_count(), 0);
    }

    #[test]
    fn a_stateless_error_counts_a_start_and_an_error() {
        let fixture = Fixture::new();
        let context = fixture.context(Some("test"));

        fixture.listener.on_error(&context, &TestError);

        let statistics = fixture.statistics("test");
        assert_eq!(statistics.get_started_count(), 1);
        assert_eq!(statistics.get_error_count(), 1);
        assert_eq!(statistics.get_complete_count(), 0);
    }

    #[test]
    fn a_stateful_error_counts_an_error_without_a_start() {
        let fixture = Fixture::new();
        let context = fixture.context(Some("test"));
        context.set_attribute(retry_context_constants::STATE_KEY, AnyValue::from("key"));

        fixture.listener.on_error(&context, &TestError);

        let statistics = fixture.statistics("test");
        assert_eq!(statistics.get_started_count(), 0);
        assert_eq!(statistics.get_error_count(), 1);
    }

    #[test]
    fn an_exhausted_block_counts_an_abort_without_a_start() {
        // The start of a stateless block is counted per error, so an exhausted
        // block adds only the abort when it closes.
        let fixture = Fixture::new();
        let context = fixture.context(Some("test"));
        context.set_attribute(retry_context_constants::EXHAUSTED, AnyValue::Boolean(true));
        context.set_attribute(retry_context_constants::CLOSED, AnyValue::Boolean(true));

        fixture.listener.close(&context, None);

        let statistics = fixture.statistics("test");
        assert_eq!(statistics.get_started_count(), 0);
        assert_eq!(statistics.get_abort_count(), 1);
        assert_eq!(statistics.get_complete_count(), 0);
    }

    #[test]
    fn an_exhausted_global_block_still_counts_a_start() {
        // With globally kept state the callback never ran, so the start has to
        // be counted here instead of per error.
        let fixture = Fixture::new();
        let context = fixture.context(Some("test"));
        context.set_attribute(retry_context_constants::EXHAUSTED, AnyValue::Boolean(true));
        context.set_attribute(
            retry_context_constants::GLOBAL_STATE,
            AnyValue::Boolean(true),
        );

        fixture.listener.close(&context, None);

        let statistics = fixture.statistics("test");
        assert_eq!(statistics.get_started_count(), 1);
        assert_eq!(statistics.get_abort_count(), 1);
    }

    #[test]
    fn a_block_that_never_closed_counts_only_a_start() {
        let fixture = Fixture::new();
        let context = fixture.context(Some("test"));

        fixture.listener.close(&context, None);

        let statistics = fixture.statistics("test");
        assert_eq!(statistics.get_started_count(), 1);
        assert_eq!(statistics.get_complete_count(), 0);
        assert_eq!(statistics.get_abort_count(), 0);
    }

    #[test]
    fn a_recovered_block_wins_over_abort_and_complete() {
        let fixture = Fixture::new();
        let context = fixture.context(Some("test"));
        context.set_attribute(retry_context_constants::EXHAUSTED, AnyValue::Boolean(true));
        context.set_attribute(retry_context_constants::RECOVERED, AnyValue::Boolean(true));
        context.set_attribute(retry_context_constants::CLOSED, AnyValue::Boolean(true));

        fixture.listener.close(&context, None);

        let statistics = fixture.statistics("test");
        assert_eq!(statistics.get_started_count(), 0);
        assert_eq!(statistics.get_recovery_count(), 1);
        assert_eq!(statistics.get_abort_count(), 0);
        assert_eq!(statistics.get_complete_count(), 0);
    }

    #[test]
    fn a_context_without_a_name_records_nothing() {
        let fixture = Fixture::new();
        let context = fixture.context(None);
        context.set_attribute(retry_context_constants::CLOSED, AnyValue::Boolean(true));

        fixture.listener.close(&context, None);
        fixture.listener.on_error(&context, &TestError);

        assert!(fixture.repository.is_empty());
    }

    #[test]
    fn the_default_name_is_used_when_the_context_has_none() {
        let fixture = Fixture::with_default_name("test");
        let context = fixture.context(None);
        context.set_attribute(retry_context_constants::CLOSED, AnyValue::Boolean(true));

        fixture.listener.close(&context, None);

        assert_eq!(fixture.listener.default_name(), Some("test"));
        assert_eq!(fixture.statistics("test").get_complete_count(), 1);
    }

    #[test]
    fn the_context_name_wins_over_the_default_name() {
        let fixture = Fixture::with_default_name("fallback");
        let context = fixture.context(Some("explicit"));
        context.set_attribute(retry_context_constants::CLOSED, AnyValue::Boolean(true));

        fixture.listener.close(&context, None);

        assert!(fixture.repository.find_one("explicit").is_some());
        assert!(fixture.repository.find_one("fallback").is_none());
    }

    #[test]
    fn a_non_string_name_attribute_falls_back_to_the_default_name() {
        let fixture = Fixture::with_default_name("fallback");
        let context = fixture.context(None);
        context.set_attribute(retry_context_constants::NAME, AnyValue::from(42_u64));
        context.set_attribute(retry_context_constants::CLOSED, AnyValue::Boolean(true));

        fixture.listener.close(&context, None);

        assert_eq!(fixture.statistics("fallback").get_complete_count(), 1);
    }

    #[test]
    fn the_listener_reports_the_repository_it_was_built_with() {
        let fixture = Fixture::new();
        let handle: Arc<dyn StatisticsRepository> = fixture.repository.clone();

        assert!(Arc::ptr_eq(fixture.listener.repository(), &handle));
    }
}
