use crate::application::key_value::KeyValue;

use super::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct HeaderRoutePredicateFactory {
    pub header: KeyValue<Option<String>>,
    pub regex: Option<regex::Regex>,
}

impl HeaderRoutePredicateFactory {}

impl RoutePredicate for HeaderRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        let key = self.header.k.as_str();
        if let Some(header_value) = session.req_header().headers.get(key) {
            if self.header.v.is_none() {
                return true;
            } else {
                return self
                    .regex
                    .as_ref()
                    .map(|reg| reg.is_match(header_value.to_str().unwrap_or("")))
                    .unwrap_or(false);
            }
        }
        false
    }
}
