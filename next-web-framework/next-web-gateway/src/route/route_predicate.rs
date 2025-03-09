use pingora::protocols::http::server::Session;

pub trait RoutePredicate {
    fn matches(&self, _session: &mut Session) -> bool;
}
