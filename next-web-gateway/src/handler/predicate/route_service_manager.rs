use hashbrown::HashMap;
use pingora::{proxy::Session, Result};
use std::sync::Arc;

use crate::{
    context::{HeaderAndBody, RequestContext},
    filter::{
        factory::local_response_cache::LocalResponseCacheFilter,
        gateway_filter::{DefaultGatewayFilter, GatewayFilter},
        gateway_filter_chain::GatewayFilterChain,
    },
    handler::DefaultGatewayFilterChain,
    properties::routes_properties::RouteMetadata,
    service::route_service::{RoutePredicateService, RouteWork},
    server::DefaultServerWebExchange,
};

use super::route_predicate_factory::RoutePredicateFactory;

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
    pub fn predicate(&self, session: &mut Session) -> RoutepRedicateResult<'_> {
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

    pub async fn filter<'a, 'b>(
        &self,
        ctx: &mut RequestContext,
        header_and_body: HeaderAndBody<'a, 'b>,
    ) -> Result<()> {
        if let Some(route_id) = &ctx.route_id {
            if let Some(service) = self.services.get(route_id) {
                let filters = service
                    .filters
                    .iter()
                    .cloned()
                    .map(|filter| Arc::new(filter) as Arc<dyn GatewayFilter>)
                    .collect();
                let chain = DefaultGatewayFilterChain::new(filters);
                let mut exchange = DefaultServerWebExchange::new(ctx, header_and_body);
                chain
                    .filter(&mut exchange)
                    .await
                    .map_err(|error| -> Box<pingora::Error> { error.into() })?;
            }
        }

        Ok(())
    }

    pub fn filter_blocking<'a, 'b>(
        &self,
        ctx: &mut RequestContext,
        header_and_body: HeaderAndBody<'a, 'b>,
    ) -> Result<()> {
        futures::executor::block_on(self.filter(ctx, header_and_body))
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
    use super::{RoutePredicateFactory, RouteServiceManager};
    use crate::context::{HeaderAndBody, RequestContext};
    use crate::filter::gateway_filter::DefaultGatewayFilter;
    use crate::service::route_service::{RoutePredicateService, RouteWork};
    use pingora::http::RequestHeader;

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

    #[test]
    fn filter_uses_chain_instead_of_polling_filters() {
        let manager = RouteServiceManager::new(vec![RoutePredicateService {
            id: "chain-route".to_string(),
            order: 0,
            work: RouteWork::Http,
            upstream: "svc-chain".to_string(),
            route_predicate_factory: Vec::new(),
            filters: vec![
                DefaultGatewayFilter::from("AddResponseHeader=X-Response:ignored"),
                DefaultGatewayFilter::from("AddRequestHeader=X-Request:passed"),
            ],
            fallback_id: String::new(),
            rate_limiter: None,
            metadata: None,
        }]);

        let mut ctx = RequestContext {
            fallback_id: None,
            route_id: Some("chain-route".to_string()),
            original_request_path: None,
            buffer_response_body: false,
            response_body_buffer: Vec::new(),
            local_response_cache_manager: None,
            local_response_cache_request: None,
            pending_local_response_cache: None,
            session: None,
            direct_response: None,
        };
        let mut request = RequestHeader::build("GET", b"/resource", None).unwrap();

        futures::executor::block_on(
            manager.filter(&mut ctx, HeaderAndBody::with_req_header(&mut request)),
        )
        .unwrap();

        assert_eq!(
            request
                .headers
                .get("x-request")
                .and_then(|value| value.to_str().ok()),
            Some("passed")
        );
    }
}
