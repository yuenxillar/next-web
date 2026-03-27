use bytes::Bytes;
use hashbrown::HashMap;
use pingora::{
    http::{RequestHeader, ResponseHeader},
    proxy::Session,
};

use crate::{
    application::next_gateway_application::ApplicationContext,
    properties::routes_properties::RouteMetadata,
    service::route_service::{RoutePredicateService, RouteWork},
};

static DEFAULT_SERVICE_NAME: &str = "";

#[derive(Clone)]
pub struct RouteServiceManager {
    services: HashMap<String, RoutePredicateService>,
    ordered_route_ids: Vec<String>,
}

impl RouteServiceManager {
    pub fn new(services: Vec<RoutePredicateService>) -> Self {
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
                    if rate_limiter.check_rate(service.id.as_str()) {
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

    pub fn filter(&self, ctx: &mut ApplicationContext, mut upstream: UpStream) {
        if let Some(route_id) = &ctx.route_id {
            if let Some(service) = self.services.get(route_id) {
                service
                    .filters
                    .iter()
                    .for_each(|filter| filter.filter(ctx, &mut upstream));
            }
        }
    }

    pub fn services(&self) -> &HashMap<String, RoutePredicateService> {
        &self.services
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
