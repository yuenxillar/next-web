use bytes::Bytes;
use hashbrown::HashMap;
use pingora::{
    http::{RequestHeader, ResponseHeader},
    proxy::Session,
    Result,
};

use crate::{
    application::next_gateway_application::ApplicationContext,
    filter::{
        gateway_filter::DefaultGatewayFilter, local_response_cache::LocalResponseCacheFilter,
    },
    properties::routes_properties::RouteMetadata,
    route::route_predicate_factory::RoutePredicateFactory,
    service::route_service::{RoutePredicateService, RouteWork},
};

static DEFAULT_SERVICE_NAME: &str = "";

#[derive(Clone)]
pub struct RouteServiceManager {
    services: HashMap<String, RoutePredicateService>,
    ordered_route_ids: Vec<String>,
}

impl RouteServiceManager {
    pub fn new(mut services: Vec<RoutePredicateService>) -> Self {
        services.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.id.cmp(&b.id)));
        configure_weight_predicates(&mut services);

        let ordered_route_ids = services.iter().map(|service| service.id.clone()).collect();
        let services = services
            .into_iter()
            .map(|service| (service.id.clone(), service))
            .collect();

        Self {
            services,
            ordered_route_ids,
        }
    }

    // var1: Predicate result
    // var2: Service name
    // var3: RouteWork
    pub fn predicate(&self, session: &mut Session) -> RoutepRedicateResult {
        for route_id in &self.ordered_route_ids {
            let Some(service) = self.services.get(route_id) else {
                continue;
            };

            let allowable = service
                .route_predicate_factory
                .iter()
                .all(|factory| factory.matches(session));

            if allowable {
                if let Some(rate_limiter) = &service.rate_limiter {
                    if rate_limiter.check_rate(&service.id) {
                        return RoutepRedicateResult {
                            allowable: false,
                            service_name: DEFAULT_SERVICE_NAME,
                            work: &RouteWork::Http,
                            fallback_id: DEFAULT_SERVICE_NAME,
                            route_id: DEFAULT_SERVICE_NAME,
                            metadata: &None,
                        };
                    }
                }

                return RoutepRedicateResult {
                    allowable,
                    service_name: &service.upstream,
                    work: &service.work,
                    fallback_id: &service.fallback_id,
                    route_id: &service.id,
                    metadata: &service.metadata,
                };
            }
        }

        RoutepRedicateResult {
            allowable: false,
            service_name: DEFAULT_SERVICE_NAME,
            work: &RouteWork::Http,
            fallback_id: DEFAULT_SERVICE_NAME,
            route_id: DEFAULT_SERVICE_NAME,
            metadata: &None,
        }
    }

    pub fn filter(&self, ctx: &mut ApplicationContext, mut upstream: UpStream) -> Result<()> {
        if let Some(route_id) = &ctx.route_id {
            if let Some(service) = self.services.get(route_id) {
                for filter in &service.filters {
                    filter.filter(ctx, &mut upstream)?;
                }
            }
        }

        Ok(())
    }

    pub fn services(&self) -> &HashMap<String, RoutePredicateService> {
        &self.services
    }

    pub fn has_response_body_filters(&self, route_id: Option<&str>) -> bool {
        let Some(route_id) = route_id else {
            return false;
        };

        self.services
            .get(route_id)
            .map(|service| {
                service
                    .filters
                    .iter()
                    .any(|filter| filter.modifies_response_body())
            })
            .unwrap_or(false)
    }

    pub fn local_response_cache_filter(
        &self,
        route_id: Option<&str>,
    ) -> Option<&LocalResponseCacheFilter> {
        let route_id = route_id?;
        let service = self.services.get(route_id)?;
        service.filters.iter().find_map(|filter| match filter {
            DefaultGatewayFilter::LocalResponseCache(filter) => Some(filter),
            _ => None,
        })
    }
}

#[derive(Debug)]
pub struct UpStream<'a, 'b> {
    pub request_header: Option<&'a mut RequestHeader>,
    pub response_header: Option<&'a mut ResponseHeader>,

    pub request_body: Option<&'b mut Bytes>,
    pub response_body: Option<&'b mut Bytes>,
}

impl<'a, 'b> UpStream<'a, 'b> {
    pub fn from_request_header(request_header: &'a mut RequestHeader) -> Self {
        Self {
            request_header: Some(request_header),
            response_header: None,
            request_body: None,
            response_body: None,
        }
    }

    pub fn from_response_header(response_header: &'a mut ResponseHeader) -> Self {
        Self {
            request_header: None,
            response_header: Some(response_header),
            request_body: None,
            response_body: None,
        }
    }

    pub fn from_request_body(request_body: &'b mut Option<Bytes>) -> Self {
        Self {
            request_header: None,
            response_header: None,
            request_body: request_body.as_mut().map(|s| s),
            response_body: None,
        }
    }

    pub fn from_response_body(response_body: &'b mut Option<Bytes>) -> Self {
        Self {
            request_header: None,
            response_header: None,
            request_body: None,
            response_body: response_body.as_mut().map(|s| s),
        }
    }
}

#[derive(Clone)]
pub struct RoutepRedicateResult<'a> {
    pub allowable: bool,
    pub service_name: &'a str,
    pub work: &'a RouteWork,
    pub fallback_id: &'a str,
    pub route_id: &'a str,
    pub metadata: &'a Option<RouteMetadata>,
}

impl Default for RouteServiceManager {
    fn default() -> Self {
        Self {
            services: HashMap::new(),
            ordered_route_ids: Vec::new(),
        }
    }
}

fn configure_weight_predicates(services: &mut [RoutePredicateService]) {
    let mut group_totals = HashMap::new();

    for service in services.iter() {
        for predicate in &service.route_predicate_factory {
            if let RoutePredicateFactory::WeightPredicates(weight) = predicate {
                let total = group_totals.entry(weight.group.clone()).or_insert(0_u32);
                *total = total.saturating_add(weight.weight);
            }
        }
    }

    let mut group_offsets = HashMap::new();
    for service in services.iter_mut() {
        for predicate in &mut service.route_predicate_factory {
            if let RoutePredicateFactory::WeightPredicates(weight) = predicate {
                let total_weight = *group_totals.get(&weight.group).unwrap_or(&0);
                let offset = group_offsets.entry(weight.group.clone()).or_insert(0_u32);
                weight.configure_range(*offset, total_weight);
                *offset = offset.saturating_add(weight.weight);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RouteServiceManager;
    use crate::route::route_predicate_factory::RoutePredicateFactory;
    use crate::service::route_service::{RoutePredicateService, RouteWork};

    #[test]
    fn new_sorts_routes_by_order_then_id() {
        let manager = RouteServiceManager::new(vec![
            RoutePredicateService {
                id: "b".to_string(),
                order: 10,
                work: RouteWork::Http,
                upstream: "svc-b".to_string(),
                route_predicate_factory: Vec::new(),
                filters: Vec::new(),
                fallback_id: String::new(),
                rate_limiter: None,
                metadata: None,
            },
            RoutePredicateService {
                id: "a".to_string(),
                order: 10,
                work: RouteWork::Http,
                upstream: "svc-a".to_string(),
                route_predicate_factory: Vec::new(),
                filters: Vec::new(),
                fallback_id: String::new(),
                rate_limiter: None,
                metadata: None,
            },
            RoutePredicateService {
                id: "c".to_string(),
                order: 5,
                work: RouteWork::Http,
                upstream: "svc-c".to_string(),
                route_predicate_factory: Vec::new(),
                filters: Vec::new(),
                fallback_id: String::new(),
                rate_limiter: None,
                metadata: None,
            },
        ]);

        assert_eq!(manager.ordered_route_ids, vec!["c", "a", "b"]);
    }

    #[test]
    fn new_assigns_weight_ranges_within_group_total() {
        let manager = RouteServiceManager::new(vec![
            RoutePredicateService {
                id: "weight-high".to_string(),
                order: 0,
                work: RouteWork::Http,
                upstream: "svc-high".to_string(),
                route_predicate_factory: vec![(&"Weight=group1,8".to_string()).into()],
                filters: Vec::new(),
                fallback_id: String::new(),
                rate_limiter: None,
                metadata: None,
            },
            RoutePredicateService {
                id: "weight-low".to_string(),
                order: 1,
                work: RouteWork::Http,
                upstream: "svc-low".to_string(),
                route_predicate_factory: vec![(&"Weight=group1,2".to_string()).into()],
                filters: Vec::new(),
                fallback_id: String::new(),
                rate_limiter: None,
                metadata: None,
            },
        ]);

        let high = manager.services().get("weight-high").unwrap();
        let low = manager.services().get("weight-low").unwrap();

        match &high.route_predicate_factory[0] {
            RoutePredicateFactory::WeightPredicates(weight) => {
                assert_eq!(weight.total_weight, 10);
                assert_eq!(weight.range_start, 0);
                assert_eq!(weight.range_end, 8);
            }
            _ => panic!("expected weight predicate"),
        }

        match &low.route_predicate_factory[0] {
            RoutePredicateFactory::WeightPredicates(weight) => {
                assert_eq!(weight.total_weight, 10);
                assert_eq!(weight.range_start, 8);
                assert_eq!(weight.range_end, 10);
            }
            _ => panic!("expected weight predicate"),
        }
    }
}
