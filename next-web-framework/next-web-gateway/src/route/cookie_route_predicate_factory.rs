use super::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct CookieRoutePredicateFactory {
    pub cookies: Vec<String>,
}

impl RoutePredicate for CookieRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        if let Some(cookie_header) = session.req_header().headers.get("Cookie") {
            if !cookie_header.is_empty() {
                if let Ok(cookie_str) = cookie_header.to_str() {
                    let result = cookie_str
                        .trim_end()
                        .split(";")
                        .collect::<Vec<&str>>()
                        .iter()
                        .any(|cookie| self.cookies.iter().any(|c| cookie.starts_with(c)));
                    if result {
                        return true;
                    }
                }
            }
        }
        false
    }
}
