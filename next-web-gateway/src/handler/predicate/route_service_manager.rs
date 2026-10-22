//! Route matching, filter application and the per-route response cache hook.

use std::collections::HashMap;
use std::sync::Arc;

use hashbrown::HashMap as FastMap;
use pingora::proxy::Session;

use crate::config::RouteMetadata;
use crate::context::{HeaderAndBody, RequestContext};
use crate::filter::factory::local_response_cache::LocalResponseCacheFilter;
use crate::filter::gateway_filter::DefaultGatewayFilter;
use crate::filter::spec::FilterPhase;
use crate::handler::DefaultGatewayFilterChain;
use crate::server::DefaultServerWebExchange;
use crate::service::route_service::{RoutePredicateService, RouteWork};

use super::route_predicate_factory::RoutePredicateFactory;

/// The route selected for one request, with everything the proxy needs.
#[derive(Debug, Clone)]
pub struct RouteMatch {
    /// Matched route id.
    pub route_id: String,
    /// Upstream authority from the route `uri`.
    pub upstream: String,
    /// How the route reaches its upstream.
    pub work: RouteWork,
    /// Circuit breaker referenced by the route, if any.
    pub fallback_id: String,
    /// Values captured by `Path` templates.
    pub path_variables: HashMap<String, String>,
    /// Route metadata.
    pub metadata: Option<RouteMetadata>,
}

/// The parsed routes plus the ordered scan used to match a request.
#[derive(Clone)]
pub struct RouteServiceManager {
    services: FastMap<String, RoutePredicateService>,
    ordered_route_ids: Vec<String>,
}

impl RouteServiceManager {
    /// Build the manager from parsed routes.
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

    /// Return the first route whose predicates all match.
    pub fn match_route(&self, session: &mut Session) -> Option<RouteMatch> {
        for route_id in &self.ordered_route_ids {
            let service = self.services.get(route_id)?;

            let allowable = service
                .route_predicate_factory
                .iter()
                .all(|factory| factory.matches(session));

            if !allowable {
                continue;
            }

            let path = session.req_header().uri.path().to_string();
            let path_variables = self.capture_variables(route_id, &path);

            return Some(RouteMatch {
                route_id: service.id.clone(),
                upstream: service.upstream.clone(),
                work: service.work.clone(),
                fallback_id: service.fallback_id.clone(),
                path_variables,
                metadata: service.metadata.clone(),
            });
        }

        None
    }

    /// Extract the `{variable}` values of a route's `Path` predicate.
    fn capture_variables(&self, route_id: &str, path: &str) -> HashMap<String, String> {
        let Some(service) = self.services.get(route_id) else {
            return HashMap::new();
        };

        service
            .route_predicate_factory
            .iter()
            .find_map(|factory| match factory {
                RoutePredicateFactory::PathPredicates(paths) => paths
                    .paths
                    .iter()
                    .find_map(|pattern| pattern.capture(path)),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// Run the filters of one phase over the exchange.
    pub async fn filter_phase<'a, 'b>(
        &self,
        ctx: &mut RequestContext,
        phase: FilterPhase,
        header_and_body: HeaderAndBody<'a, 'b>,
    ) -> pingora::Result<()> {
        let Some(route_id) = ctx.route_id.as_deref() else {
            return Ok(());
        };
        let Some(service) = self.services.get(route_id) else {
            return Ok(());
        };

        let chain = DefaultGatewayFilterChain::new(DefaultGatewayFilter::chain_for(
            &service.filters,
            phase,
        ));
        let mut exchange = DefaultServerWebExchange::new(ctx, header_and_body);

        crate::filter::gateway_filter_chain::GatewayFilterChain::filter(&chain, &mut exchange)
            .await
            .map_err(|error| -> Box<pingora::Error> { error.into() })
    }

    /// Apply the response body filters of the matched route, in place.
    pub fn apply_body_filters(&self, ctx: &RequestContext, body: &mut bytes::Bytes) {
        let Some(route_id) = ctx.route_id.as_deref() else {
            return;
        };
        let Some(service) = self.services.get(route_id) else {
            return;
        };

        for filter in &service.filters {
            filter.apply_body(body);
        }
    }

    /// Routes, for the admin endpoint.
    pub fn services(&self) -> &FastMap<String, RoutePredicateService> {
        &self.services
    }

    /// Route ids in evaluation order.
    pub fn ordered_route_ids(&self) -> &[String] {
        &self.ordered_route_ids
    }

    /// Whether the route rewrites the response body.
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

    /// The response cache filter configured on the route, if any.
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

impl Default for RouteServiceManager {
    fn default() -> Self {
        Self {
            services: FastMap::new(),
            ordered_route_ids: Vec::new(),
        }
    }
}

/// Assign every `Weight` predicate its slice of the group's bucket space.
fn configure_weight_predicates(services: &mut [RoutePredicateService]) {
    let mut group_totals = FastMap::new();

    for service in services.iter() {
        for predicate in &service.route_predicate_factory {
            if let RoutePredicateFactory::WeightPredicates(weight) = predicate {
                let total = group_totals.entry(weight.group.clone()).or_insert(0_u32);
                *total = total.saturating_add(weight.weight);
            }
        }
    }

    let mut group_offsets = FastMap::new();
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

/// Shared handle so health checks and the admin endpoint can share routes.
pub type SharedRoutes = Arc<RouteServiceManager>;

#[cfg(test)]
mod tests {
    use super::RouteServiceManager;
    use crate::config::RouteConfig;
    use crate::filter::gateway_filter::DefaultGatewayFilter;
    use crate::service::route_service::RoutePredicateService;

    fn route(id: &str, order: i32, uri: &str) -> RouteConfig {
        RouteConfig {
            id: id.to_string(),
            uri: uri.to_string(),
            order,
            predicates: Vec::new(),
            filters: Vec::new(),
            upstreams: Vec::new(),
            metadata: Default::default(),
        }
    }

    #[test]
    fn new_sorts_routes_by_order_then_id() {
        let routes = vec![route("b", 10, "http://h:1"), route("a", 10, "http://h:1"), route("c", 5, "http://h:1")];
        let services = routes
            .into_iter()
            .map(|route| RoutePredicateService::try_from(route).unwrap())
            .collect();

        let manager = RouteServiceManager::new(services);
        assert_eq!(manager.ordered_route_ids(), vec!["c", "a", "b"]);
    }

    #[test]
    fn filters_are_split_by_phase() {
        let request = DefaultGatewayFilter::try_from("AddRequestHeader=X-A, 1").unwrap();
        let response = DefaultGatewayFilter::try_from("AddResponseHeader=X-B, 1").unwrap();

        assert_eq!(
            DefaultGatewayFilter::chain_for(&[request.clone(), response.clone()], crate::filter::spec::FilterPhase::RequestHeader).len(),
            1
        );
        assert_eq!(
            DefaultGatewayFilter::chain_for(&[request, response], crate::filter::spec::FilterPhase::ResponseHeader).len(),
            1
        );
    }
}
