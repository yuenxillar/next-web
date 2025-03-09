use super::route_predicate::RoutePredicate;
use pingora::http::Method;

#[derive(Debug, Clone)]
pub struct MethodRoutePredicateFactory {
    pub methods: Vec<Method>,
}

impl RoutePredicate for MethodRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        for method in &self.methods {
            if method == &session.req_header().method {
                return true;
            }
        }
        false
    }
}
