use std::sync::Arc;

use axum::{extract::Request, response::Response};
use next_web_core::error::BoxError;

use crate::core::filter::Filter;

use super::{
    firewall::{
        http_firewall::HttpFirewall,
        http_status_request_rejected_handler::HttpStatusRequestRejectedHandler,
        request_rejected_handler::RequestRejectedHandler, strict_http_firewall::StrictHttpFirewall,
    },
    security_filter_chain::SecurityFilterChain,
};

#[derive(Clone)]
pub struct FilterChainProxy {
    pub(crate) filter_chains: Vec<Arc<dyn SecurityFilterChain>>,
    pub(crate) filter_chain_validator: Arc<dyn FilterChainValidator>,
    pub(crate) firewall: Arc<dyn HttpFirewall>,
    pub(crate) request_rejected_handler: Arc<dyn RequestRejectedHandler>,
}

impl FilterChainProxy {
    pub fn new(filter_chains: Vec<Arc<dyn SecurityFilterChain>>) -> Self {
        Self {
            filter_chains,
            filter_chain_validator: Arc::new(NullFilterChainValidator::default()),
            firewall: Arc::new(StrictHttpFirewall::default()),
            request_rejected_handler: Arc::new(HttpStatusRequestRejectedHandler::default()),
        }
    }
}

impl Filter for FilterChainProxy {
    fn do_filter(&self, req: &mut Request, res: &mut Response) -> Result<(), BoxError> {
        self.filter_chain_validator.validate(self);

        let Some(chain) = self
            .filter_chains
            .iter()
            .find(|chain| chain.matches(req))
            .cloned()
        else {
            return Ok(());
        };

        for filter in chain.get_filters() {
            filter.do_filter(req, res)?;
            if !res.status().is_success() && !res.status().is_redirection() {
                break;
            }
        }

        Ok(())
    }
}

pub trait FilterChainValidator: Send + Sync {
    fn validate(&self, filter_chain_proxy: &FilterChainProxy);
}

#[derive(Clone, Default)]
pub struct NullFilterChainValidator;

impl FilterChainValidator for NullFilterChainValidator {
    fn validate(&self, _filter_chain_proxy: &FilterChainProxy) {}
}
