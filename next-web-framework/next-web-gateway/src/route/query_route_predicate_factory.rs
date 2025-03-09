use super::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct QueryRoutePredicateFactory {
    pub name: String,
}

impl RoutePredicate for QueryRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        if let Some(querys) = session.req_header().uri.query() {
            let result = querys
                .split("&")
                .collect::<Vec<&str>>()
                .iter()
                .any(|f| f.starts_with(&self.name));
            if result {
                return true;
            }
        }
        false
    }
}
