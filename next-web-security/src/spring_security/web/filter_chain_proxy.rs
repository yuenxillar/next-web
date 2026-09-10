use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use tracing::{enabled, Level};

use super::{
    firewall::{
        http_firewall::HttpFirewall,
        http_status_request_rejected_handler::HttpStatusRequestRejectedHandler,
        request_rejected_handler::RequestRejectedHandler, strict_http_firewall::StrictHttpFirewall,
    },
    security_filter_chain::SecurityFilterChain,
};
use crate::core::context::{SecurityContextHolder, SecurityContextHolderStrategy};

#[derive(Clone)]
pub struct FilterChainProxy {
    pub(crate) filter_chains: Vec<Arc<dyn SecurityFilterChain>>,
    pub(crate) filter_chain_validator: Arc<dyn FilterChainValidator>,
    pub(crate) firewall: Arc<dyn HttpFirewall>,
    pub(crate) request_rejected_handler: Arc<dyn RequestRejectedHandler>,
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    validated: Arc<AtomicBool>,
}

impl FilterChainProxy {
    pub fn new(filter_chains: Vec<Arc<dyn SecurityFilterChain>>) -> Self {
        Self {
            filter_chains,
            filter_chain_validator: Arc::new(NullFilterChainValidator::default()),
            firewall: Arc::new(StrictHttpFirewall::default()),
            request_rejected_handler: Arc::new(HttpStatusRequestRejectedHandler::default()),
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            validated: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Return the configured security filter chains in first-match order.
    pub fn filter_chains(&self) -> &[Arc<dyn SecurityFilterChain>] {
        &self.filter_chains
    }

    /// Replace the request firewall used to validate incoming requests.
    pub fn set_firewall(&mut self, firewall: Arc<dyn HttpFirewall>) {
        self.firewall = firewall;
    }

    /// Replace the handler invoked when the firewall rejects a request.
    pub fn set_request_rejected_handler(&mut self, handler: Arc<dyn RequestRejectedHandler>) {
        self.request_rejected_handler = handler;
    }

    /// Replace the validator and arrange for the new validator to run once.
    pub fn set_filter_chain_validator(&mut self, validator: Arc<dyn FilterChainValidator>) {
        self.filter_chain_validator = validator;
        self.validated.store(false, Ordering::Release);
    }

    /// Configure the strategy cleared when the outermost proxy invocation ends.
    pub fn set_security_context_holder_strategy(
        &mut self,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = strategy;
    }

    fn validate_once(&self) {
        if self
            .validated
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.filter_chain_validator.validate(self);
        }
    }
}

#[async_trait]
impl HttpFilter for FilterChainProxy {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        self.validate_once();
        let applied = request.get_attribute(FILTER_APPLIED).is_some();
        if !applied {
            request.set_attribute(
                FILTER_APPLIED,
                next_web_core::anys::any_value::AnyValue::Boolean(true),
            );
        }
        let result = match self.firewall.validate_request(request) {
            Ok(()) => {
                let mut firewall_request = self.firewall.get_firewalled_request(request);
                let firewall_response = self.firewall.get_firewalled_response(response);
                let chain = self
                    .filter_chains
                    .iter()
                    .find(|chain| chain.matches(firewall_request.request_mut()));
                let result = match chain {
                    Some(chain) if !chain.get_filters().is_empty() => {
                        let virtual_chain =
                            VirtualFilterChain::new(chain.get_filters(), filter_chain);
                        virtual_chain
                            .do_filter(firewall_request.request_mut(), &mut **firewall_response)
                            .await
                    }
                    _ => {
                        filter_chain
                            .do_filter(firewall_request.request_mut(), &mut **firewall_response)
                            .await
                    }
                };
                firewall_request.reset();
                result
            }
            Err(error) => self
                .request_rejected_handler
                .handle(request, response, &error)
                .map_err(|e| FilterError::from(e)),
        };
        if !applied {
            // Spring clears the holder for every outermost invocation, even if
            // a firewall or an internal filter returned an error.
            self.security_context_holder_strategy.clear_context();
            request.remove_attribute(FILTER_APPLIED);
        }
        result
    }
}

const FILTER_APPLIED: &str = "web.FilterChainProxy.APPLIED";

/// Internal chain that advances one security filter at a time and delegates to
/// the original application chain after the last filter.
#[derive(Clone)]
pub struct VirtualFilterChain {
    filters: Arc<Vec<Arc<dyn HttpFilter>>>,
    original: Box<dyn HttpFilterChain>,
    position: Arc<AtomicUsize>,
}

impl VirtualFilterChain {
    pub fn new(filters: &[Arc<dyn HttpFilter>], original: &dyn HttpFilterChain) -> Self {
        Self {
            filters: Arc::new(filters.to_vec()),
            original: next_web_core::clone_box(original),
            position: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl HttpFilterChain for VirtualFilterChain {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        let index = self.position.fetch_add(1, Ordering::AcqRel);
        if let Some(filter) = self.filters.get(index) {
            if enabled!(Level::TRACE) {
                tracing::trace!(
                    "Invoking {} ({}/{})",
                    filter.name(),
                    index + 1,
                    self.filters.len()
                );
            }
            filter.do_filter(request, response, self).await
        } else {
            self.original.do_filter(request, response).await
        }
    }
}

impl Named for FilterChainProxy {
    fn name(&self) -> &str {
        "FilterChainProxy"
    }
}

/// A strategy for decorating the provided filter chain with one that accounts for the SecurityFilterChain for a given request.
pub trait FilterChainDecorator
where
    Self: Send + Sync,
{
    /// Provide a new FilterChain that accounts for needed security considerations when there are no security filters.
    fn decorate(&self, original: Box<dyn HttpFilterChain>) -> Box<dyn HttpFilterChain> {
        self.decorate_with_filters(original, Vec::new())
    }

    /// Provide a new FilterChain that accounts for the provided filters as well as the original filter chain.
    fn decorate_with_filters(
        &self,
        original: Box<dyn HttpFilterChain>,
        filters: Vec<Arc<dyn HttpFilter>>,
    ) -> Box<dyn HttpFilterChain>;
}

pub trait FilterChainValidator
where
    Self: Send + Sync,
{
    fn validate(&self, _filter_chain_proxy: &FilterChainProxy);
}

#[derive(Clone, Default)]
pub struct NullFilterChainValidator;

impl FilterChainValidator for NullFilterChainValidator {
    fn validate(&self, _filter_chain_proxy: &FilterChainProxy) {}
}
