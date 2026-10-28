//! `Header` predicate: matches when any value of a header matches.

use regex::Regex;

use super::route_predicate::RoutePredicate;
use crate::util::key_value::KeyValue;

/// The `Header` predicate.
#[derive(Debug, Clone)]
pub struct HeaderRoutePredicateFactory {
    /// Header name in `k`, the configured pattern in `v` (informational).
    pub header: KeyValue<Option<String>>,
    /// Anchored pattern compiled at configuration time.
    pub regex: Option<Regex>,
}

impl RoutePredicate for HeaderRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        let values = session
            .req_header()
            .headers
            .get_all(self.header.k.as_ref())
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect::<Vec<_>>();

        matches_values(&values, self.regex.as_ref())
    }
}

/// Whether any of the header values satisfies the predicate.
///
/// A predicate without a pattern only requires the header to be present, which
/// is also what `Header=X-Test,` means in the configuration.
fn matches_values(values: &[&str], regex: Option<&Regex>) -> bool {
    if values.is_empty() {
        return false;
    }

    match regex {
        None => true,
        Some(regex) => values.iter().any(|value| regex.is_match(value)),
    }
}

#[cfg(test)]
mod tests {
    use super::matches_values;
    use regex::Regex;

    #[test]
    fn header_name_only_matches_when_present() {
        assert!(matches_values(&["anything"], None));
        assert!(!matches_values(&[], None));
    }

    #[test]
    fn header_matches_any_of_several_values() {
        let regex = Regex::new("^(?:token-)").unwrap();
        assert!(matches_values(&["other", "token-abc"], Some(&regex)));
        assert!(!matches_values(&["other", "second"], Some(&regex)));
    }

    #[test]
    fn header_regex_is_fully_anchored() {
        let regex = Regex::new("^(?:abc)$").unwrap();
        assert!(matches_values(&["abc"], Some(&regex)));
        assert!(!matches_values(&["xabc"], Some(&regex)));
    }
}
