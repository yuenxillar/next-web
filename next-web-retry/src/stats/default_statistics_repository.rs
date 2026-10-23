//! The default name-keyed statistics repository.

use std::sync::Arc;

use dashmap::DashMap;

use crate::{
    retry_policy::RetryPolicy,
    stats::{
        default_retry_statistics_factory::DefaultRetryStatisticsFactory,
        mutable_retry_statistics::MutableRetryStatistics,
        retry_statistics_factory::RetryStatisticsFactory,
        statistics_repository::StatisticsRepository,
    },
};

/// Thread-safe [`StatisticsRepository`] keyed by retry block name.
///
/// # Keying by name
///
/// The policy instance is the natural identity of a retry block, but a borrowed
/// Rust trait object has no comparable identity, so this repository keys by the
/// name a policy reports. Two policies that report the same name therefore share
/// one statistics instance; see [`StatisticsRepository`] for the trade-off.
///
/// # Concrete statistics type
///
/// New entries are created by a [`RetryStatisticsFactory`], which is the seam
/// that decides which concrete statistics type is used. The default factory
/// produces [`ExponentialAverageRetryStatistics`], so a repository created with
/// [`DefaultStatisticsRepository::new`] offers both the plain counters and the
/// rolling averages. Because the factory chooses the type, entries are stored as
/// `Arc<dyn MutableRetryStatistics>`; storing one concrete statistics type would
/// stop a custom factory from contributing its own.
///
/// [`ExponentialAverageRetryStatistics`]: crate::stats::ExponentialAverageRetryStatistics
pub struct DefaultStatisticsRepository {
    statistics: DashMap<String, Arc<dyn MutableRetryStatistics>>,
    factory: Arc<dyn RetryStatisticsFactory>,
}

impl DefaultStatisticsRepository {
    /// Creates a repository that uses the default statistics factory.
    pub fn new() -> Self {
        Self::with_factory(Arc::new(DefaultRetryStatisticsFactory::default()))
    }

    /// Creates a repository that creates entries with `factory`.
    pub fn with_factory(factory: Arc<dyn RetryStatisticsFactory>) -> Self {
        Self {
            statistics: DashMap::new(),
            factory,
        }
    }

    /// Returns the number of stored statistics instances.
    pub fn len(&self) -> usize {
        self.statistics.len()
    }

    /// Returns `true` when no statistics instance has been created yet.
    pub fn is_empty(&self) -> bool {
        self.statistics.is_empty()
    }
}

impl Default for DefaultStatisticsRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl StatisticsRepository for DefaultStatisticsRepository {
    fn get_statistics(&self, policy: &dyn RetryPolicy) -> Arc<dyn MutableRetryStatistics> {
        self.get_statistics_by_name(policy.name())
    }

    fn get_statistics_by_name(&self, name: &str) -> Arc<dyn MutableRetryStatistics> {
        // The common case is a hit: clone the handle and drop the shard guard
        // immediately so that nothing is held while the factory runs.
        if let Some(existing) = self.statistics.get(name) {
            return Arc::clone(&existing);
        }

        // A miss may race with another writer; `or_insert_with` keeps the
        // instance that won, so all callers share one value per name. The
        // losing instance is dropped without ever being observed.
        let created = self.factory.create(name);
        let entry = self
            .statistics
            .entry(name.to_owned())
            .or_insert_with(|| created);

        Arc::clone(&entry)
    }

    fn find_one(&self, name: &str) -> Option<Arc<dyn MutableRetryStatistics>> {
        self.statistics.get(name).map(|value| Arc::clone(&value))
    }

    fn find_all(&self) -> Vec<Arc<dyn MutableRetryStatistics>> {
        self.statistics
            .iter()
            .map(|value| Arc::clone(&value))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use next_web_core::Named;

    use crate::{
        context::retry_context_support::RetryContextSupport,
        error::retry_error::RetryError,
        retry_context::RetryContext,
        retry_policy::RetryPolicy,
        stats::{
            default_retry_statistics::DefaultRetryStatistics,
            default_statistics_repository::DefaultStatisticsRepository,
            mutable_retry_statistics::MutableRetryStatistics,
            retry_statistics_factory::RetryStatisticsFactory,
            statistics_repository::StatisticsRepository,
        },
    };

    /// Policy with an explicit name, so a test can decide which names collide.
    struct TestPolicy {
        name: &'static str,
    }

    impl RetryPolicy for TestPolicy {
        fn can_retry(&self, _context: &dyn RetryContext) -> bool {
            true
        }

        fn open(&self, _parent: Option<Arc<dyn RetryContext>>) -> Arc<dyn RetryContext> {
            Arc::new(RetryContextSupport::new())
        }

        fn close(&self, _context: &dyn RetryContext) {}

        fn register_error(&self, _context: &dyn RetryContext, _error: &RetryError) {}
    }

    impl Named for TestPolicy {
        fn name(&self) -> &str {
            self.name
        }
    }

    /// Factory that always creates plain statistics, used to prove the factory
    /// seam is honoured.
    struct PlainStatisticsFactory;

    impl RetryStatisticsFactory for PlainStatisticsFactory {
        fn create(&self, name: &str) -> Arc<dyn MutableRetryStatistics> {
            Arc::new(DefaultRetryStatistics::new(name))
        }
    }

    #[test]
    fn the_same_policy_reuses_one_statistics_instance() {
        let repository = DefaultStatisticsRepository::new();
        let policy = TestPolicy { name: "test" };

        let first = repository.get_statistics(&policy);
        let second = repository.get_statistics(&policy);

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(repository.len(), 1);

        first.increment_started_count();
        assert_eq!(second.get_started_count(), 1);
    }

    #[test]
    fn policies_sharing_a_name_share_one_statistics_instance() {
        let repository = DefaultStatisticsRepository::new();

        let first = repository.get_statistics(&TestPolicy { name: "shared" });
        let second = repository.get_statistics(&TestPolicy { name: "shared" });

        assert_eq!(first.name(), "shared");
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(repository.len(), 1);
    }

    #[test]
    fn policies_with_distinct_names_get_distinct_instances() {
        let repository = DefaultStatisticsRepository::new();

        let first = repository.get_statistics(&TestPolicy { name: "first" });
        let second = repository.get_statistics(&TestPolicy { name: "second" });

        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(repository.len(), 2);
    }

    #[test]
    fn looked_up_entries_are_created_on_first_use_only() {
        let repository = DefaultStatisticsRepository::new();

        assert!(repository.is_empty());
        assert!(repository.find_one("missing").is_none());
        assert!(repository.is_empty());

        repository.add_started("test");

        assert_eq!(repository.len(), 1);
        let statistics = repository.get_statistics_by_name("test");
        assert_eq!(statistics.name(), "test");
        assert_eq!(statistics.get_started_count(), 1);
        assert!(repository.find_one("test").is_some());
    }

    #[test]
    fn every_event_is_recorded_on_the_same_instance() {
        let repository = DefaultStatisticsRepository::new();

        repository.add_started("test");
        repository.add_error("test");
        repository.add_error("test");
        repository.add_complete("test");
        repository.add_abort("test");
        repository.add_recovery("test");

        let statistics = repository.get_statistics_by_name("test");
        assert_eq!(statistics.get_started_count(), 1);
        assert_eq!(statistics.get_error_count(), 2);
        assert_eq!(statistics.get_complete_count(), 1);
        assert_eq!(statistics.get_abort_count(), 1);
        assert_eq!(statistics.get_recovery_count(), 1);
    }

    #[test]
    fn find_all_returns_a_snapshot_of_every_instance() {
        let repository = DefaultStatisticsRepository::new();

        repository.add_started("first");
        repository.add_started("second");

        let all = repository.find_all();
        assert_eq!(all.len(), 2);

        repository.add_started("third");
        assert_eq!(all.len(), 2);
        assert_eq!(repository.find_all().len(), 3);
    }

    #[test]
    fn a_custom_factory_controls_the_concrete_statistics_type() {
        let repository =
            DefaultStatisticsRepository::with_factory(Arc::new(PlainStatisticsFactory));

        let statistics = repository.get_statistics_by_name("test");
        statistics.increment_abort_count();

        assert_eq!(statistics.get_abort_count(), 1);
        assert_eq!(repository.len(), 1);
    }
}
