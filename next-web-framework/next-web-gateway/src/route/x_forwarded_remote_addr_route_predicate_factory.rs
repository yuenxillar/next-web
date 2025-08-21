use crate::route::route_predicate::RoutePredicate;


#[derive(Debug, Clone)]
pub struct XForwardedRemoteAddrRoutePredicateFactory {
    pub allowed_ips: Vec<String>,
}

impl RoutePredicate for XForwardedRemoteAddrRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        todo!()
    }
}
