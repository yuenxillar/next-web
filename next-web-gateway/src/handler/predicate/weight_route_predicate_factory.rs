//! `Weight` predicate: distributes a route group across weighted buckets.

use super::route_predicate::RoutePredicate;

/// The `Weight` predicate.
#[derive(Debug, Clone)]
pub struct WeightRoutePredicateFactory {
    /// Route group; every route in a group shares the bucket space.
    pub group: String,
    /// Relative weight of this route inside the group.
    pub weight: u32,
    /// First bucket owned by this route.
    pub range_start: u32,
    /// One past the last bucket owned by this route.
    pub range_end: u32,
    /// Sum of the weights of the whole group.
    pub total_weight: u32,
}

impl WeightRoutePredicateFactory {
    /// Build a predicate for a group and a weight.
    pub fn new(group: String, weight: u32) -> Self {
        Self {
            group,
            weight,
            range_start: 0,
            range_end: 0,
            total_weight: 0,
        }
    }

    /// Assign this route the half open bucket window `range_start..range_start+weight`.
    pub fn configure_range(&mut self, range_start: u32, total_weight: u32) {
        self.total_weight = total_weight;
        self.range_start = range_start;
        self.range_end = range_start.saturating_add(self.weight);
    }

    /// Whether a freshly drawn bucket belongs to this route.
    pub fn owns_bucket(&self, bucket: u32) -> bool {
        self.weight > 0
            && self.total_weight > 0
            && self.range_start < self.range_end
            && self.range_start <= bucket
            && bucket < self.range_end
    }
}

impl RoutePredicate for WeightRoutePredicateFactory {
    fn matches(&self, _session: &mut pingora::protocols::http::ServerSession) -> bool {
        if self.weight == 0 || self.total_weight == 0 || self.range_start >= self.range_end {
            return false;
        }

        // Spring re-randomises for every evaluation, so the split is a stable
        // ratio over many requests rather than a sticky per-client choice.
        self.owns_bucket(random_bucket(self.total_weight))
    }
}

/// Draw a bucket uniformly from `0..total_weight`.
fn random_bucket(total_weight: u32) -> u32 {
    if total_weight == 0 {
        return 0;
    }

    rand::random::<u32>() % total_weight
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{random_bucket, WeightRoutePredicateFactory};

    #[test]
    fn configure_range_sets_expected_window() {
        let mut predicate = WeightRoutePredicateFactory::new("group1".to_string(), 8);
        predicate.configure_range(2, 10);

        assert_eq!(predicate.range_start, 2);
        assert_eq!(predicate.range_end, 10);
        assert_eq!(predicate.total_weight, 10);
    }

    #[test]
    fn bucket_is_bounded_by_the_group_weight() {
        for _ in 0..1_000 {
            assert!(random_bucket(10) < 10);
        }
    }

    #[test]
    fn bucket_is_randomised_per_request() {
        let buckets: HashSet<u32> = (0..200).map(|_| random_bucket(10)).collect();
        assert!(
            buckets.len() > 1,
            "expected several distinct buckets, got {buckets:?}"
        );
    }

    #[test]
    fn a_route_only_owns_its_window() {
        let mut predicate = WeightRoutePredicateFactory::new("group".to_string(), 2);
        predicate.configure_range(8, 10);

        assert!(predicate.owns_bucket(8));
        assert!(predicate.owns_bucket(9));
        assert!(!predicate.owns_bucket(7));
        assert!(!predicate.owns_bucket(10));
    }
}
