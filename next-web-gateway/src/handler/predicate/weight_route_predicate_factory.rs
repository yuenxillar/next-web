use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct WeightRoutePredicateFactory {
    pub group: String,
    pub weight: u32,
    pub range_start: u32,
    pub range_end: u32,
    pub total_weight: u32,
}

impl WeightRoutePredicateFactory {
    pub fn new(group: String, weight: u32) -> Self {
        Self {
            group,
            weight,
            range_start: 0,
            range_end: 0,
            total_weight: 0,
        }
    }

    pub fn configure_range(&mut self, range_start: u32, total_weight: u32) {
        self.total_weight = total_weight;
        self.range_start = range_start;
        self.range_end = range_start.saturating_add(self.weight);
    }
}

impl RoutePredicate for WeightRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        if self.weight == 0 || self.total_weight == 0 || self.range_start >= self.range_end {
            return false;
        }

        let request_key = request_hash_key(session, &self.group);
        let Some(bucket) = bucket_for_key(&request_key, self.total_weight) else {
            return false;
        };

        self.range_start <= bucket && bucket < self.range_end
    }
}

fn request_hash_key(session: &pingora::protocols::http::ServerSession, group: &str) -> String {
    let header = session.req_header();
    let method = header.method.as_str();
    let path_and_query = header
        .uri
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or_else(|| header.uri.path());
    let host = header
        .headers
        .get("Host")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let remote_addr = session
        .client_addr()
        .map(|addr| format!("{addr:?}"))
        .unwrap_or_default();

    format!("{group}|{method}|{host}|{path_and_query}|{remote_addr}")
}

fn bucket_for_key(key: &str, total_weight: u32) -> Option<u32> {
    if total_weight == 0 {
        return None;
    }

    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    Some((hasher.finish() % total_weight as u64) as u32)
}

#[cfg(test)]
mod tests {
    use super::{bucket_for_key, WeightRoutePredicateFactory};

    #[test]
    fn configure_range_sets_expected_window() {
        let mut predicate = WeightRoutePredicateFactory::new("group1".to_string(), 8);
        predicate.configure_range(2, 10);

        assert_eq!(predicate.range_start, 2);
        assert_eq!(predicate.range_end, 10);
        assert_eq!(predicate.total_weight, 10);
    }

    #[test]
    fn bucket_for_key_is_stable_and_bounded() {
        let first = bucket_for_key("group1|GET|example.com|/test|127.0.0.1", 10).unwrap();
        let second = bucket_for_key("group1|GET|example.com|/test|127.0.0.1", 10).unwrap();

        assert_eq!(first, second);
        assert!(first < 10);
    }
}
