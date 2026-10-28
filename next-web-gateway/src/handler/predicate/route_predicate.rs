use pingora::protocols::http::server::Session;

pub(crate) trait RoutePredicate {
    fn matches(&self, session: &mut Session) -> bool;
}
