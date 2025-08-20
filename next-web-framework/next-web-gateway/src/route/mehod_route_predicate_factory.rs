use std::collections::HashSet;

use super::route_predicate::RoutePredicate;
use pingora::http::Method;

#[derive(Debug, Clone)]
pub struct MethodRoutePredicateFactory {
    pub methods: HashSet<Method>,
}

impl RoutePredicate for MethodRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        self.methods.contains(&session.req_header().method)
    }
}
