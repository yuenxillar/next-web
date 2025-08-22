use std::sync::Arc;

use pingora::{
    http::{RequestHeader, ResponseHeader},
    proxy::Session,
};
use pingora_limits::rate::Rate;

use crate::{
    application::next_gateway_application::ApplicationContext,
    filter::gateway_filter::DefaultGatewayFilter,
    properties::routes_properties::{RouteMetadata, RoutesProperties},
    rate_limiter::{RateLimiter, RATE_KEY},
};

use super::route_predicate_factory::RoutePredicateFactory;

static DEFAULT_SERVICE_NAME: &str = "";

#[derive(Clone)]
pub struct RouteServiceManager {
    services: Vec<RoutePredicateService>,
}

impl RouteServiceManager {
    pub fn new(services: Vec<RoutePredicateService>) -> Self {
        Self { services }
    }

    // var1: Predicate result
    // var2: Service name
    // var3: RouteWork
    pub fn predicate(&self, session: &mut Session) -> RoutepRedicateResult {
        for service in self.services.iter() {
            let result = service
                .route_predicate_factory
                .iter()
                .all(|f| f.matches(session));

            if result {
                // Rate Limiter implementation
                if let Some(rate_limiter) = &service.rate_limiter {
                    if rate_limiter.check_rate() {
                        return RoutepRedicateResult {
                            result: false,
                            service_name: DEFAULT_SERVICE_NAME,
                            work: &RouteWork::Http,
                            fallback_id: DEFAULT_SERVICE_NAME,
                            route_id: DEFAULT_SERVICE_NAME,
                            metadata: &None
                        };

                    } else {
                        rate_limiter.rate.observe(&RATE_KEY, 1);
                    }
                }
                return RoutepRedicateResult {
                    result,
                    service_name: &service.upstream,
                    work: &service.work,
                    fallback_id: &service.fallback_id,
                    route_id: &service.id,
                    metadata: &service.metadata
                };
            }
        }

        RoutepRedicateResult {
            result: false,
            service_name: DEFAULT_SERVICE_NAME,
            work: &RouteWork::Http,
            fallback_id: DEFAULT_SERVICE_NAME,
            route_id: DEFAULT_SERVICE_NAME,
            metadata: &None
        }
    }

    pub fn filter(
        &self,
        ctx: &mut ApplicationContext,
        upstream_request_header: &mut RequestHeader,
        upstream_response_header: &mut ResponseHeader,
    ) {
        if let Some(route_id) = &ctx.route_id {
            self.services()
                .iter()
                .find(|s| s.id.eq(route_id))
                .map(|sevice| {
                    sevice.filters.iter().for_each(|f| {
                        f.filter(ctx, upstream_request_header, upstream_response_header)
                    })
                });
        }
    }

    pub fn services(&self) -> &Vec<RoutePredicateService> {
        &self.services
    }
}

#[derive(Clone)]
pub struct RoutepRedicateResult<'a> {
    pub result: bool,
    pub service_name: &'a str,
    pub work: &'a RouteWork,
    pub fallback_id: &'a str,
    pub route_id: &'a str,
    pub metadata: &'a Option<RouteMetadata>,
}

impl Default for RouteServiceManager {
    fn default() -> Self {
        Self {
            services: Vec::new(),
        }
    }
}

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
            .map(|filter| filter.into())
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
            metadata
        }
    }
}
