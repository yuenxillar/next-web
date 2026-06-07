use std::sync::Arc;

use next_web_core::{
    async_trait,
    error::BoxError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

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

#[async_trait]
impl HttpFilter for FilterChainProxy {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        self.filter_chain_validator.validate(self);

        let Some(chain) = self
            .filter_chains
            .iter()
            .find(|chain| chain.matches(request))
            .cloned()
        else {
            return Ok(());
        };

        for filter in chain.get_filters() {
            filter.do_filter(request, response, filter_chain).await?;
            if !response.status_code().is_success() && !response.status_code().is_redirection() {
                break;
            }
        }

        Ok(())
    }
}

impl Named for FilterChainProxy {
    fn name(&self) -> &str {
        "FilterChainProxy"
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
