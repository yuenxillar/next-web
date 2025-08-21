use pingora::protocols::http::server::Session;

pub trait RoutePredicate {
    fn matches(&self, session: &mut Session) -> bool;
}
