//! A parsed route: predicates, filters and the upstream it proxies to.

use std::sync::Arc;

use tracing::debug;

use crate::config::{ConfigError, ConfigErrors, RouteConfig, RouteMetadata};
use crate::filter::gateway_filter::{DefaultGatewayFilter, GatewayFilter};
use crate::filter::spec::FilterPhase;
use crate::handler::predicate::RoutePredicateFactory;
use crate::util::rate_limiter::RateLimiter;

/// One route, ready to be evaluated and applied.
#[derive(Clone)]
pub struct RoutePredicateService {
    /// Unique route identifier.
    pub id: String,
    /// Evaluation order, lower runs first.
    pub order: i32,
    /// How the route reaches its upstream.
    pub work: RouteWork,
    /// Upstream authority, without the scheme.
    pub upstream: String,
    /// Predicates that must all match.
    pub route_predicate_factory: Vec<RoutePredicateFactory>,
    /// Filters applied in declaration order.
    pub filters: Vec<DefaultGatewayFilter>,
    /// `RequestHeader` filters, built once so a request never re-filters or
    /// re-clones the filter list.
    pub request_header_chain: Arc<[Arc<dyn GatewayFilter>]>,
    /// `ResponseHeader` filters.
    pub response_header_chain: Arc<[Arc<dyn GatewayFilter>]>,
    /// `ResponseBody` filters.
    pub response_body_chain: Arc<[Arc<dyn GatewayFilter>]>,
    /// Circuit breaker referenced by the route, if any.
    pub fallback_id: String,
    /// Route level rate limiter, migrated into a filter when configured.
    pub rate_limiter: Option<RateLimiter>,
    /// Optional per-route metadata.
    pub metadata: Option<RouteMetadata>,
}

impl RoutePredicateService {
    /// The prebuilt chain for `phase`.
    pub fn chain_for(&self, phase: FilterPhase) -> Arc<[Arc<dyn GatewayFilter>]> {
        match phase {
            FilterPhase::RequestHeader => self.request_header_chain.clone(),
            FilterPhase::ResponseHeader => self.response_header_chain.clone(),
            FilterPhase::ResponseBody => self.response_body_chain.clone(),
        }
    }
}

/// How a route reaches its upstream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteWork {
    /// Plain HTTP to a fixed authority.
    Http,
    /// TLS to a fixed authority.
    Https,
    /// Load balanced across the configured upstreams.
    LB,
}

impl From<&str> for RouteWork {
    fn from(uri: &str) -> Self {
        let scheme = uri
            .split_once("://")
            .map(|(scheme, _)| scheme)
            .unwrap_or(uri);

        match scheme.to_ascii_lowercase().as_str() {
            "https" => Self::Https,
            "lb" => Self::LB,
            _ => Self::Http,
        }
    }
}

impl TryFrom<RouteConfig> for RoutePredicateService {
    type Error = ConfigErrors;

    fn try_from(route: RouteConfig) -> Result<Self, Self::Error> {
        let mut errors: Vec<ConfigError> = Vec::new();

        let predicates = route
            .predicates
            .iter()
            .filter_map(
                |predicate| match RoutePredicateFactory::try_from(predicate.as_str()) {
                    Ok(factory) => Some(factory),
                    Err(error) => {
                        errors.push(error);
                        None
                    }
                },
            )
            .collect::<Vec<_>>();

        let filters = route
            .filters
            .iter()
            .filter_map(
                |filter| match DefaultGatewayFilter::try_from(filter.as_str()) {
                    Ok(filter) => Some(filter),
                    Err(error) => {
                        errors.push(error);
                        None
                    }
                },
            )
            .collect::<Vec<_>>();

        if !errors.is_empty() {
            return Err(ConfigErrors::new(errors));
        }

        // `CircuitBreaker=<id>` binds the route to a breaker; the proxy enforces
        // it, so the id is lifted out of the parsed filter list here.
        let fallback_id = filters
            .iter()
            .find_map(|filter| match filter {
                DefaultGatewayFilter::CircuitBreaker(filter) => Some(filter.breaker_id.clone()),
                _ => None,
            })
            .unwrap_or_default();

        debug!("route `{}` parsed with {} filters", route.id, filters.len());

        let work = RouteWork::from(route.uri.as_str());
        let upstream = route
            .uri
            .split_once("://")
            .map(|(_, authority)| authority.to_owned())
            .unwrap_or_else(|| route.uri.clone());

        Ok(Self {
            id: route.id,
            order: route.order,
            work,
            upstream,
            route_predicate_factory: predicates,
            request_header_chain: Arc::from(DefaultGatewayFilter::chain_for(
                &filters,
                FilterPhase::RequestHeader,
            )),
            response_header_chain: Arc::from(DefaultGatewayFilter::chain_for(
                &filters,
                FilterPhase::ResponseHeader,
            )),
            response_body_chain: Arc::from(DefaultGatewayFilter::chain_for(
                &filters,
                FilterPhase::ResponseBody,
            )),
            filters,
            fallback_id,
            rate_limiter: None,
            metadata: Some(route.metadata),
        })
    }
}
