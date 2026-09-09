use std::sync::Arc;

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    core::context::{SecurityContextHolder, SecurityContextHolderStrategy},
    web::context::SecurityContextRepository,
};

#[derive(Clone)]
pub struct SecurityContextHolderFilter {
    security_context_repository: Arc<dyn SecurityContextRepository>,
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
}

impl SecurityContextHolderFilter {
    const FILTER_APPLIED: &str = "SecurityContextHolderFilter.APPLIED";

    pub fn new(security_context_repository: Arc<dyn SecurityContextRepository>) -> Self {
        Self {
            security_context_repository,
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
        }
    }

    /// Sets the SecurityContextHolderStrategy to use.
    /// The default action is to use the SecurityContextHolderStrategy stored in SecurityContextHolder.
    pub fn set_security_context_holder_strategy(
        &mut self,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = security_context_holder_strategy;
    }
}

#[async_trait]
impl HttpFilter for SecurityContextHolderFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.get_attribute(Self::FILTER_APPLIED).is_some() {
            return filter_chain.do_filter(request, response).await;
        }

        request.set_attribute(Self::FILTER_APPLIED, AnyValue::Boolean(true));

        let context = self
            .security_context_repository
            .load_deferred_context(request);
        self.security_context_holder_strategy
            .set_deferred_context(Arc::new(move || context.get()));

        let result = filter_chain.do_filter(request, response).await;
        self.security_context_holder_strategy.clear_context();
        request.remove_attribute(Self::FILTER_APPLIED);
        result
    }
}

impl Named for SecurityContextHolderFilter {
    fn name(&self) -> &str {
        "SecurityContextHolderFilter"
    }
}
