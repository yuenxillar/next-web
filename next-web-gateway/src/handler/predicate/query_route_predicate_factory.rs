//! `Query` predicate: matches on the first query parameter with that name.

use form_urlencoded::parse;
use regex::Regex;

use super::route_predicate::RoutePredicate;

/// The `Query` predicate.
#[derive(Debug, Clone)]
pub struct QueryRoutePredicateFactory {
    /// Query parameter name.
    pub name: String,
    /// Anchored pattern compiled at configuration time.
    pub regex: Option<Regex>,
}

impl RoutePredicate for QueryRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        let Some(query) = session.req_header().uri.query() else {
            return false;
        };

        query_matches(query, &self.name, self.regex.as_ref())
    }
}

fn query_matches(query: &str, name: &str, regex: Option<&Regex>) -> bool {
    let Some(value) = parse(query.as_bytes())
        .find_map(|(key, value)| (key.as_ref() == name).then(|| value.into_owned()))
    else {
        return false;
    };

    match regex {
        None => true,
        Some(regex) => regex.is_match(&value),
    }
}

#[cfg(test)]
mod tests {
    use super::query_matches;
    use regex::Regex;

    #[test]
    fn query_matches_name_only() {
        assert!(query_matches("token=abc&lang=zh", "token", None));
        assert!(!query_matches("lang=zh", "token", None));
    }

    #[test]
    fn query_matches_regex_value() {
        let regex = Regex::new(r"^\d+$").unwrap();
        assert!(query_matches("page=123", "page", Some(&regex)));
        assert!(!query_matches("page=abc", "page", Some(&regex)));
    }

    #[test]
    fn query_uses_the_first_value_with_that_name() {
        let regex = Regex::new("^(?:first)$").unwrap();
        assert!(query_matches("q=first&q=second", "q", Some(&regex)));
    }
}
