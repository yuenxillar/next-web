use regex::Regex;

use super::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct CookieRoutePredicateFactory {
    pub name: String,
    pub regex: Option<Regex>,
}

impl RoutePredicate for CookieRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        let Some(cookie_header) = session.req_header().headers.get("Cookie") else {
            return false;
        };
        let Ok(cookie_str) = cookie_header.to_str() else {
            return false;
        };

        cookie_matches(cookie_str, &self.name, self.regex.as_ref())
    }
}

/// Match the first cookie with the given name against the optional pattern.
fn cookie_matches(cookie_header: &str, name: &str, regex: Option<&Regex>) -> bool {
    let Some(value) = cookie_header
        .split(';')
        .filter_map(|cookie| cookie.trim().split_once('='))
        .find_map(|(cookie_name, cookie_value)| {
            (cookie_name.trim() == name).then(|| cookie_value.trim())
        })
    else {
        return false;
    };

    match regex {
        None => true,
        Some(regex) => regex.is_match(value),
    }
}

#[cfg(test)]
mod tests {
    use super::cookie_matches;
    use regex::Regex;

    #[test]
    fn cookie_matches_name_only() {
        assert!(cookie_matches("session=abc; theme=dark", "session", None));
        assert!(!cookie_matches("theme=dark", "session", None));
    }

    #[test]
    fn cookie_matches_regex_value() {
        let regex = Regex::new(r"^ab").unwrap();
        assert!(cookie_matches("session=abc", "session", Some(&regex)));
        assert!(!cookie_matches("session=xyz", "session", Some(&regex)));
    }

    #[test]
    fn cookie_uses_the_first_value_with_that_name() {
        let regex = Regex::new("^(?:first)$").unwrap();
        assert!(cookie_matches(
            "session=first; session=second",
            "session",
            Some(&regex)
        ));
    }
}
