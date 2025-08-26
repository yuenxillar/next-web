use std::sync::Arc;

use pingora_limits::rate::Rate;

use crate::properties::routes_properties::RoutesProperties;
use crate::route::route_predicate_factory::RoutePredicateFactory;
use crate::util::rate_limiter::RateLimiter;
use crate::{
    filter::gateway_filter::DefaultGatewayFilter, properties::routes_properties::RouteMetadata,
};

#[derive(Clone)]
pub struct RoutePredicateService {
    pub id: String,
    pub order: i32,
    pub work: RouteWork,
    pub upstream: String,
    pub route_predicate_factory: Vec<RoutePredicateFactory>,
    pub filters: Vec<DefaultGatewayFilter>,
    pub fallback_id: String,
    pub rate_limiter: Option<RateLimiter>,
    pub metadata: Option<RouteMetadata>,
}

#[derive(Debug, Clone)]
pub enum RouteWork {
    Http,
    Https,
    LB,
}

impl Into<RouteWork> for &str {
    fn into(self) -> RouteWork {
        if self.starts_with("http") {
            return RouteWork::Http;
        } else if self.starts_with("lb") {
            return RouteWork::LB;
        } else if self.starts_with("https") {
            return RouteWork::Https;
        }
        return RouteWork::Http;
    }
}

impl From<RoutesProperties> for RoutePredicateService {
    fn from(routes_properties: RoutesProperties) -> Self {
        let work = routes_properties.uri().into();
        let upstream = routes_properties
            .uri()
            .split("://")
            .collect::<Vec<&str>>()
            .get(1)
            .map(|s| s.to_string())
            .unwrap_or(routes_properties.uri().into());

        // Get RoutePredicateFactory from RoutesProperties
        let route_predicate_factory = routes_properties
            .predicates()
            .iter()
            .map(|predicate| predicate.into())
            .filter(|factory| {
                if let &RoutePredicateFactory::Nothing = factory {
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<RoutePredicateFactory>>();

        // Get Filters from RoutesProperties
        let filters = routes_properties
            .filters()
            .iter()
            .map(|s| DefaultGatewayFilter::from(s.as_ref()))
            .collect::<Vec<DefaultGatewayFilter>>();
        println!("filters: {:#?}", filters);
        // Sort filters by order
        // filters.sort_by(|a, b| match (a, b) {
        //     (GatewayFilter::CircuitBreaker(_), _) => std::cmp::Ordering::Less,
        //     (_, GatewayFilter::CircuitBreaker(_)) => std::cmp::Ordering::Greater,
        //     _ => std::cmp::Ordering::Equal,
        // });

        let order = routes_properties.order.unwrap_or(i32::MAX);
        let fallback_id = routes_properties
            .filters
            .iter()
            .find(|s| s.starts_with("CircuitBreaker"))
            .map(|s| s.split_once('=').unwrap())
            .map(|s| s.1.to_string())
            .unwrap_or_default();

        let num = routes_properties.rate_limiter.unwrap_or(0);
        let rate_limiter = if num == 0 {
            None
        } else {
            Some(RateLimiter {
                limit: num as f64,
                rate: Arc::new(Rate::new(std::time::Duration::from_secs(1))),
            })
        };

        let id = routes_properties.id().to_string();
        let metadata = routes_properties.metadata;

        Self {
            id,
            upstream,
            order,
            work,
            route_predicate_factory,
            filters,
            fallback_id,
            rate_limiter,
            metadata,
        }
    }
}
