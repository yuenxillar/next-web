use chrono::DateTime;
use chrono_tz::Tz;

use super::route_predicate::RoutePredicate;

/// A factory that creates a route predicate that checks if the current time is within a certain time zone.
#[derive(Debug, Clone)]
pub struct ZonedDateTimeRoutePredicateFactory {
    pub datetime: Vec<DateTime<Tz>>,
    pub offset: u8,
}

impl RoutePredicate for ZonedDateTimeRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        false
    }
}
