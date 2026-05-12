use form_urlencoded::parse;
use regex::Regex;

use super::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct QueryRoutePredicateFactory {
    pub name: String,
    pub regex: Option<Regex>,
}

impl RoutePredicate for QueryRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        let Some(query_str) = session.req_header().uri.query() else {
            return false;
        };

        query_matches(query_str, &self.name, self.regex.as_ref())
    }
}

fn query_matches(query: &str, name: &str, regex: Option<&Regex>) -> bool {
    parse(query.as_bytes()).any(|(key, value)| {
        if key.as_ref() != name {
            return false;
        }

        regex
            .map(|compiled| compiled.is_match(value.as_ref()))
            .unwrap_or(true)
    })
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
}
