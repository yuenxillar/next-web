use super::route_predicate::RoutePredicate;
use matchit::Router;

#[derive(Debug, Clone)]
pub struct PathRoutePredicateFactory {
    pub paths: Router<bool>,
}

impl RoutePredicate for PathRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        let path = session.req_header().raw_path();
        if let Ok(result) = self.paths.at(&String::from_utf8_lossy(path)) {
            if *result.value {
                return true;
            }
        }
        false
    }
}
