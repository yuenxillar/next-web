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

fn cookie_matches(cookie_header: &str, name: &str, regex: Option<&Regex>) -> bool {
    cookie_header
        .split(';')
        .filter_map(|cookie| cookie.trim().split_once('='))
        .any(|(cookie_name, cookie_value)| {
            if cookie_name.trim() != name {
                return false;
            }

            regex
                .map(|compiled| compiled.is_match(cookie_value.trim()))
                .unwrap_or(true)
        })
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
}
